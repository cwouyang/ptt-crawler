use std::ops::Range;

use regex::Regex;
use reqwest::{Client, RedirectPolicy};
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
pub fn create_client() -> Result<Client, Error> {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(RedirectPolicy::none())
        .build()
        .or_else(|e| Err(Error::ConnectionError(e)))?;

    let params = [("yes", "yes")];
    let url = format!("{}/ask/over18", PTT_CC_URL);
    client
        .post(&url)
        .form(&params)
        .send()
        .map(|_| Ok(client))
        .or_else(|e| Err(Error::ConnectionError(e)))?
}

/// Crawl the page count of given board.
pub fn crawl_page_count(client: &Client, board: &BoardName) -> Result<u32, Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"index(?P<num>\d+)").unwrap();
    }

    let latest_page_url = compose_page_url(&board, 0);
    let document = transform_to_document(&client, &latest_page_url)?;
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
    Ok(count_until_last_page + 1)
}

/// Given a URL, crawls the page and parses it into an Article.
pub fn crawl_url(client: &Client, url: &str) -> Result<Article, Error> {
    if !is_supported_url(url) {
        return Err(Error::InvalidUrl);
    }

    let document = transform_to_document(client, url)?;
    parser::parse(&document).map_err(|_| Error::InvalidResponse)
}

/// Given a board, crawls specified pages and parses them into Articles.
pub fn crawl_pages(
    client: &Client,
    board: BoardName,
    range: Range<u32>,
) -> Result<Vec<Article>, Error> {
    let mut articles: Vec<Article> = vec![];
    for page in range {
        let page_url = compose_page_url(&board, page);
        match crawl_one_page(client, &page_url) {
            Ok(mut a) => articles.append(&mut a),
            Err(e) => error!("Error {:?} occurred when parsing {:?}", e, page_url),
        };
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

fn transform_to_document(client: &Client, url: &str) -> Result<Document, Error> {
    match client.get(url).send() {
        Ok(mut r) => {
            if !r.status().is_success() {
                return Err(Error::InvalidResponse);
            }
            let text = r.text().unwrap();
            Ok(Document::from(text.as_str()))
        }
        Err(e) => Err(Error::ConnectionError(e)),
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

fn crawl_one_page(client: &Client, url: &str) -> Result<Vec<Article>, Error> {
    info!("Start crawling page {}", url);
    let document = transform_to_document(client, url)?;
    let articles = document
        .find(Class("title"))
        .flat_map(|n| {
            n.children().find(|n| {
                let title = n.text();
                n.name() == Some("a") && !title.trim().is_empty()
            })
        })
        .map(|a| a.attr("href").unwrap().to_owned())
        .filter_map(|relative_path| {
            let article_url = format!("{}{}", PTT_CC_URL, relative_path);
            info!("Start crawling article {}", article_url);
            match crawl_url(client, &article_url) {
                Ok(article) => {
                    info!("Succeeded!");
                    Some(article)
                }
                Err(e) => {
                    error!("Failed! {:?}", e);
                    None
                }
            }
        })
        .collect::<Vec<Article>>();
    info!("Succeeded to crawl page");
    Ok(articles)
}

#[cfg(test)]
mod tests {
    use super::*;

    lazy_static! {
        static ref CLIENT: Client = create_client().unwrap();
    }

    #[test]
    fn test_crawl_not_ptt_url() {
        assert!(match crawl_url(&CLIENT, "https://www.google.com") {
            Err(e) => match e {
                Error::InvalidUrl => true,
                _ => false,
            },
            _ => false,
        });
    }

    #[test]
    fn test_crawl_invalid_ptt_url() {
        assert!(match crawl_url(&CLIENT, "https://www.ptt.cc") {
            Err(e) => match e {
                Error::InvalidUrl => true,
                _ => false,
            },
            _ => false,
        });
    }

    #[test]
    fn test_crawl_none_exist_ptt_url() {
        assert!(
            match crawl_url(&CLIENT, "https://www.ptt.cc/bbs/Gossiping/M.html") {
                Err(e) => match e {
                    Error::InvalidResponse => true,
                    _ => false,
                },
                _ => false,
            }
        );
    }
}
