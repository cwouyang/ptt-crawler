use reqwest::{Client, RedirectPolicy};
use select::document::Document;
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
pub fn create_client() -> Result<Client, Error> {
    let client = match reqwest::Client::builder()
        .cookie_store(true)
        .redirect(RedirectPolicy::none())
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err(Error::ConnectionError(e)),
    };

    let params = [("yes", "yes")];
    let url = format!("{}/ask/over18", PTT_CC_URL);
    match client.post(&url).form(&params).send() {
        Ok(_) => Ok(client),
        Err(e) => return Err(Error::ConnectionError(e)),
    }
}

/// Given a URL, crawls the page and parses it into an Article
pub fn crawl(client: &Client, url: &str) -> Result<Article, Error> {
    if !is_supported_url(url) {
        return Err(Error::InvalidUrl);
    }

    let document = match client.get(url).send() {
        Ok(mut r) => {
            if !r.status().is_success() {
                return Err(Error::InvalidResponse);
            }
            let text = r.text().unwrap();
            Document::from(text.as_str())
        }
        Err(e) => return Err(Error::ConnectionError(e)),
    };
    parser::parse(&document)
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

#[cfg(test)]
mod tests {
    use super::*;

    lazy_static! {
        static ref CLIENT: Client = create_client().unwrap();
    }

    #[test]
    fn test_crawl_not_ptt_url() {
        assert!(match crawl(&CLIENT, "https://www.google.com") {
            Err(e) => match e {
                Error::InvalidUrl => true,
                _ => false,
            },
            _ => false,
        });
    }

    #[test]
    fn test_crawl_invalid_ptt_url() {
        assert!(match crawl(&CLIENT, "https://www.ptt.cc") {
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
            match crawl(&CLIENT, "https://www.ptt.cc/bbs/Gossiping/M.html") {
                Err(e) => match e {
                    Error::InvalidResponse => true,
                    _ => false,
                },
                _ => false,
            }
        );
    }
}
