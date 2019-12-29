use reqwest::{Client, RedirectPolicy};
use select::document::Document;

use crate::{article::Article, parser};

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
    let document = match client.get(url).send() {
        Ok(mut r) => {
            let text = r.text().unwrap();
            Document::from(text.as_str())
        }
        Err(e) => return Err(Error::ConnectionError(e)),
    };
    parser::parse(&document)
}
