use std::boxed::Box;
use std::ops::RangeInclusive;

use regex::Regex;
use reqwest::{redirect::Policy, Client, Proxy};
use select::document::Document;
use select::predicate::{Class, Name, Predicate};
use url::Url;

use crate::{article::Article, article::BoardName, parser};

const PTT_CC_URL: &str = "https://www.ptt.cc";

/// Error represents the errors which might occur when crawling.
#[derive(Debug)]
pub enum Error {
    ConnectionError(reqwest::Error),
    InvalidUrl,
    InvalidResponse,
}

/// Return a HTTP Client with cookie accepting over 18 agreement.
/// One should reuse returned client as more as possible.
pub async fn create_client(proxies: Option<Vec<Proxy>>) -> Result<Client, Error> {
    let mut builder = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(Policy::none());
    if let Some(mut proxy) = proxies {
        while !proxy.is_empty() {
            builder = builder.proxy(proxy.pop().unwrap())
        }
    }
    let client = builder
        .build()
        .or_else(|e| Err(Error::ConnectionError(e)))?;

    let params = [("yes", "yes")];
    let url = format!("{}/ask/over18", PTT_CC_URL);
    match client.post(&url).form(&params).send().await {
        Ok(_) => Ok(client),
        Err(e) => Err(Error::ConnectionError(e)),
    }
}

/// Crawl the page count of given board.
pub async fn crawl_page_count(client: &Client, board: &BoardName) -> Result<u32, Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"index(?P<num>\d+)").unwrap();
    }

    info!("Start crawling page count of board {}", board);
    let latest_page_url = compose_page_url(&board, 0);
    let document = transform_to_document(client, &latest_page_url).await?;
    let last_page_url = match document
        .find(Name("a").and(Class("wide")))
        .find(|n| n.text() == "‹ 上頁")
    {
        Some(n) => n.attr("href").unwrap(),
        None => return Ok(1),
    };
    let count_until_last_page = RE
        .captures(last_page_url)
        .unwrap()
        .name("num")
        .unwrap()
        .as_str()
        .parse::<u32>()
        .unwrap();
    let page_count = count_until_last_page + 1;
    info!(
        "Finish crawling page count of board {}. page_count: {}",
        board, page_count
    );
    Ok(page_count)
}

/// Given a URL, crawls the page and parses it into an Article.
pub async fn crawl_url(client: &Client, url: &str) -> Result<Article, Error> {
    info!("Start crawling article with URL {}", url);
    if !is_supported_url(url) {
        error!("not supported URL {}", url);
        return Err(Error::InvalidUrl);
    }

    let document = transform_to_document(client, url).await?;
    let result = parser::parse(&document).map_err(|_| Error::InvalidResponse);
    info!("Finish crawling article with URL {}", url);
    result
}

/// Given a board, crawls and returns the URLs of articles within range.
pub async fn crawl_page_urls(
    client: &Client,
    board: &BoardName,
    range: &RangeInclusive<u32>,
) -> Result<Vec<String>, Error> {
    info!(
        "Start crawling URLs of articles from board {} page {} to {}",
        board,
        range.start(),
        range.end()
    );
    let mut article_urls: Vec<String> = vec![];
    let mut error: Error = Error::InvalidResponse;
    for page_num in range.clone() {
        let page_url = compose_page_url(&board, page_num);
        match crawl_one_page_urls(client, &page_url).await {
            Ok(mut urls) => article_urls.append(&mut urls),
            Err(e) => {
                error!("{:?} occurred when crawling {}", e, page_url);
                error = e;
            }
        };
    }

    info!(
        "Finish crawling URLs of articles from board {} page {} to {}",
        board,
        range.start(),
        range.end()
    );
    if article_urls.is_empty() {
        error!("No URL was found");
        return Err(error);
    }
    Ok(article_urls)
}

/// Given a board, crawls and returns parsed Articles within range.
pub async fn crawl_page_articles(
    client: &Client,
    board: &BoardName,
    range: &RangeInclusive<u32>,
) -> Result<Vec<Article>, Error> {
    info!(
        "Start crawling articles from board {} page {} to {}",
        board,
        range.start(),
        range.end()
    );
    let mut articles: Vec<Article> = vec![];
    let mut error: Error = Error::InvalidResponse;
    let article_urls = crawl_page_urls(client, board, range).await?;
    for url in article_urls {
        match crawl_url(client, &url).await {
            Ok(article) => articles.push(article),
            Err(e) => {
                error!("{:?} occurred when crawling {:?}", e, url);
                error = e;
            }
        }
    }

    info!(
        "Finish crawling articles from board {} page {} to {}",
        board,
        range.start(),
        range.end()
    );
    if articles.is_empty() {
        error!("No article was found");
        return Err(error);
    }
    Ok(articles)
}

fn is_supported_url(url: &str) -> bool {
    if !url.contains(PTT_CC_URL) {
        return false;
    }

    let ptt_cc_url_valid_path: Vec<Box<dyn Fn(&str) -> bool>> = {
        vec![
            Box::new(move |s| s == "bbs"),
            Box::new(move |s| s.to_owned().parse::<BoardName>().is_ok()),
        ]
    };

    let parsed_url = match Url::parse(url) {
        Ok(url) => url,
        Err(_) => return false,
    };
    let segments = parsed_url.path_segments().unwrap();
    segments
        .zip(ptt_cc_url_valid_path.iter())
        .fold(true, |ok, (segment, predicate)| ok && predicate(segment))
}

async fn transform_to_document(client: &Client, url: &str) -> Result<Document, Error> {
    let text_future = match client.get(url).send().await {
        Ok(r) => {
            if !r.status().is_success() {
                return Err(Error::InvalidResponse);
            }
            r.text()
        }
        Err(e) => return Err(Error::ConnectionError(e)),
    };
    match text_future.await {
        Ok(t) => Ok(Document::from(t.as_str())),
        Err(_) => Err(Error::InvalidResponse),
    }
}

fn compose_page_url(board: &BoardName, page: u32) -> String {
    format!(
        "{}/bbs/{}/index{}.html",
        PTT_CC_URL,
        board.to_string(),
        page
    )
}

async fn crawl_one_page_urls(client: &Client, url: &str) -> Result<Vec<String>, Error> {
    info!("Start crawling article URLs in page {}", url);
    let document = transform_to_document(client, url).await?;
    let article_urls = document
        .find(Class("title"))
        .flat_map(|n| {
            n.children().find(|n| {
                let title = n.text();
                n.name() == Some("a") && !title.trim().is_empty()
            })
        })
        .map(|a| {
            let relative_path = a.attr("href").unwrap().to_owned();
            format!("{}{}", PTT_CC_URL, relative_path)
        })
        .collect();
    info!("Finish crawling article URLs in page {}", url);
    Ok(article_urls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_crawl_not_ptt_url() {
        let client = create_client(None).await.unwrap();

        assert!(match crawl_url(&client, "https://www.google.com").await {
            Err(e) => match e {
                Error::InvalidUrl => true,
                _ => false,
            },
            _ => false,
        });
    }

    #[tokio::test]
    async fn test_crawl_invalid_ptt_url() {
        let client = create_client(None).await.unwrap();

        assert!(match crawl_url(&client, "https://www.ptt.cc").await {
            Err(e) => match e {
                Error::InvalidUrl => true,
                _ => false,
            },
            _ => false,
        });
    }

    #[tokio::test]
    async fn test_crawl_none_exist_ptt_url() {
        let client = create_client(None).await.unwrap();

        assert!(
            match crawl_url(&client, "https://www.ptt.cc/bbs/Gossiping/M.html").await {
                Err(e) => match e {
                    Error::InvalidResponse => true,
                    _ => false,
                },
                _ => false,
            }
        );
    }
}
