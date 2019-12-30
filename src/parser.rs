use std::net::Ipv4Addr;

use chrono::{offset::FixedOffset, offset::LocalResult, prelude::*, DateTime};
use regex::Regex;
use select::predicate::{Attr, Class, Name, Predicate};
use select::{document::Document, node::Node};

use crate::article::{Article, BoardName, Reply, ReplyCount, ReplyType};
use crate::crawler;

lazy_static! {
    static ref TW_TIME_OFFSET: FixedOffset = FixedOffset::east(8 * 3600);
}

/// Error represents the errors which might occur when parsing.
#[derive(Debug)]
pub enum Error {
    InvalidFormat,
    FieldNotFound,
}

pub fn parse(document: &Document) -> Result<Article, crawler::Error> {
    if !is_article_exist(&document) {
        return Err(crawler::Error::InvalidResponse);
    }

    let (id, category, title, author_id, author_name, board, date, ip) = parse_meta(&document);
    let content = parse_content(&document);
    let replies = parse_replies(&document, &date);

    let reply_count = ReplyCount {
        push: replies
            .iter()
            .filter(|r| r.reply_type == ReplyType::Push)
            .count() as u16,
        neutral: replies
            .iter()
            .filter(|r| r.reply_type == ReplyType::Neutral)
            .count() as u16,
        boo: replies
            .iter()
            .filter(|r| r.reply_type == ReplyType::Boo)
            .count() as u16,
    };
    return Ok(Article {
        id,
        category,
        title,
        author_id,
        author_name,
        content,
        board,
        date,
        ip,
        reply_count,
        replies,
    });
}

fn is_article_exist(document: &Document) -> bool {
    !document
        .find(Class("bbs-content"))
        .any(|n: Node| n.text().contains("404 - Not Found."))
}

fn parse_meta(
    document: &Document,
) -> (
    String,
    String,
    String,
    String,
    Option<String>,
    BoardName,
    DateTime<FixedOffset>,
    Ipv4Addr,
) {
    let id = parse_id(document);
    let (category, title) = match parse_title(document) {
        (Some(category), title) => (category, title),
        (None, title) => ("".to_owned(), title),
    };
    let (author_id, author_name) = parse_author(document);
    let board = parse_board(document);
    let date = parse_date(document, &TW_TIME_OFFSET)
        .unwrap_or(Local::now().with_timezone(&TW_TIME_OFFSET));
    let ip = parse_ip(document);

    (id, category, title, author_id, author_name, board, date, ip)
}

fn parse_id(document: &Document) -> String {
    let url = document
        .find(Name("link").and(Attr("rel", "canonical")))
        .nth(0)
        .unwrap()
        .attr("href")
        .unwrap();
    let split_url = url.split("/").collect::<Vec<_>>();
    let mut id = split_url.last().unwrap().to_owned();
    let html_extension_index: usize = id.find(".html").unwrap();
    id = &id[..html_extension_index];
    id.to_owned()
}

fn parse_title(document: &Document) -> (Option<String>, String) {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"((\[|［)(?P<category>\w+)+(\]|］)\s*)?(?P<title>.+)").unwrap();
    }

    let original_title = document
        .find(Name("meta").and(Attr("property", "og:title")))
        .nth(0)
        .unwrap()
        .attr("content")
        .unwrap();
    match RE.captures(original_title) {
        Some(cap) => (
            match cap.name("category") {
                Some(m) => Some(m.as_str().to_owned()),
                None => None,
            },
            cap["title"].to_owned(),
        ),
        None => (None, original_title.to_owned()),
    }
}

fn parse_author(document: &Document) -> (String, Option<String>) {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<id>\w+)\s\((?P<name>.+)\)").unwrap();
    }

    let author = document
        .find(Name("span").and(Class("article-meta-value")))
        .nth(0)
        .unwrap()
        .text();
    match RE.captures(&author) {
        Some(cap) => (cap["id"].to_owned(), Some(cap["name"].to_owned())),
        None => (author.to_owned(), None),
    }
}

fn parse_board(document: &Document) -> BoardName {
    let board = document
        .find(Name("span").and(Class("article-meta-value")))
        .nth(1)
        .unwrap()
        .inner_html();
    board.parse::<BoardName>().unwrap_or(BoardName::Unknown)
}

fn parse_date(
    document: &Document,
    fixed_offset: &FixedOffset,
) -> Result<DateTime<FixedOffset>, Error> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"(?P<date>\w{3} \w{3} \d{2} \d{2}:\d{2}:\d{2} \d{4})").unwrap();
    }

    let time_str = match document
        .find(Name("span").and(Class("article-meta-value")))
        .nth(3)
    {
        Some(node) => node.text(),
        None => {
            let main_content = document
                .find(Name("div").and(Attr("id", "main-content")))
                .nth(0)
                .unwrap()
                .text();
            match RE.captures(&main_content) {
                Some(cap) => cap["date"].to_owned(),
                None => return Err(Error::FieldNotFound),
            }
        }
    };

    match NaiveDateTime::parse_from_str(&time_str, "%a %b  %e %H:%M:%S %Y") {
        Ok(time) => match fixed_offset.from_local_datetime(&time) {
            LocalResult::Single(offset_time) => Ok(offset_time),
            _ => Err(Error::InvalidFormat),
        },
        Err(_) => Err(Error::InvalidFormat),
    }
}

fn parse_ip(document: &Document) -> Ipv4Addr {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<ip>\d{1,3}.\d{1,3}.\d{1,3}.\d{1,3})").unwrap();
    }

    let s = match document
        .find(Name("span").and(Class("f2")))
        .map(|n| n.inner_html())
        .find(|html| html.contains("發信站: 批踢踢實業坊(ptt.cc), 來自:"))
    {
        Some(ip) => ip,
        None => {
            let main_content = document
                .find(Name("div").and(Attr("id", "main-content")))
                .nth(0)
                .unwrap()
                .text();
            let sub_content_start_index =
                main_content.find("發信站: 批踢踢實業坊(ptt.cc)").unwrap();
            main_content[sub_content_start_index..].to_owned()
        }
    };

    match RE.captures(&s) {
        Some(cap) => cap["ip"].parse::<Ipv4Addr>().unwrap(),
        None => Ipv4Addr::new(0, 0, 0, 0),
    }
}

fn parse_content(document: &Document) -> String {
    let main_content = document
        .find(Name("div").and(Attr("id", "main-content")))
        .nth(0)
        .unwrap()
        .text();
    let content_start_index = main_content.find('\n').unwrap();
    let content_end_index = main_content.find("--\n※").unwrap();
    let content = &main_content[content_start_index..content_end_index];
    content.trim().to_owned()
}

fn parse_replies(document: &Document, article_time: &DateTime<FixedOffset>) -> Vec<Reply> {
    document
        .find(Name("div").and(Class("push")))
        .flat_map(|n| parse_reply(&n, article_time.year()))
        .collect::<Vec<Reply>>()
}

fn parse_reply(node: &Node, article_year: i32) -> Result<Reply, Error> {
    if node.text() == "檔案過大！部分文章無法顯示" {
        return Err(Error::InvalidFormat);
    }

    let fixed_offset = FixedOffset::east(8 * 3600);
    let reply_type = node
        .find(Name("span").and(Class("push-tag")))
        .nth(0)
        .unwrap()
        .text()
        .trim()
        .parse::<ReplyType>()
        .unwrap();
    let author_id = node
        .find(Name("span").and(Class("push-userid")))
        .nth(0)
        .unwrap()
        .text();
    let content = node
        .find(Name("span").and(Class("push-content")))
        .nth(0)
        .unwrap()
        .text()
        .trim_start_matches(|c| (c == ':' || c == ' '))
        .to_owned();
    let time = node
        .find(Name("span").and(Class("push-ipdatetime")))
        .nth(0)
        .unwrap()
        .text();
    let time_with_year = format!("{}/{}", article_year, time.trim());
    let date = match NaiveDateTime::parse_from_str(&time_with_year, "%Y/%m/%d %H:%M") {
        Ok(time) => match fixed_offset.from_local_datetime(&time) {
            LocalResult::Single(offset_time) => offset_time,
            _ => Local::now().with_timezone(&fixed_offset),
        },
        Err(_) => Local::now().with_timezone(&fixed_offset),
    };

    Ok(Reply {
        author_id,
        reply_type,
        content,
        date,
    })
}

#[cfg(test)]
mod tests {
    use select::document::Document;

    use super::*;

    fn load_document(path: &str) -> Document {
        Document::from(load_str!(path))
    }

    #[test]
    fn test_deleted_article() {
        let documents = load_document("../tests/Gossiping_M.1577579359.A.B76.html");

        assert!(match parse(&documents) {
            Ok(_) => false,
            Err(e) => match e {
                crawler::Error::InvalidResponse => true,
                _ => false,
            },
        });
    }

    #[test]
    fn test_parse_id() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");

        assert_eq!(parse_id(&documents), "M.1181801925.A.86E".to_owned());
    }

    #[test]
    fn test_parse_title_with_category() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");

        assert_eq!(
            parse_title(&documents),
            (Some("公告".to_owned()), "Soft_Job 板試閱".to_owned())
        );
    }

    #[test]
    fn test_parse_title_without_category() {
        let documents = load_document("../tests/Soft_Job_M.1181803258.A.666.html");
        assert_eq!(parse_title(&documents), (None, "搶頭香".to_owned()));
    }

    #[test]
    fn test_parse_title_with_ascii_char() {
        let documents = load_document("../tests/Soft_Job_M.1181804025.A.7A7.html");
        assert_eq!(parse_title(&documents), (None, "恭喜開板 ^^".to_owned()));
    }

    #[test]
    fn test_parse_title_as_reply() {
        let documents = load_document("../tests/Soft_Job_M.1181804025.A.7A7.html");
        assert_eq!(parse_title(&documents), (None, "恭喜開板 ^^".to_owned()));
    }

    #[test]
    fn test_parse_author() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");

        assert_eq!(
            parse_author(&documents),
            ("Junchoon".to_owned(), Some("裘髯客".to_owned()))
        );
    }

    #[test]
    fn test_parse_author_with_non_chinese() {
        let documents = load_document("../tests/Soft_Job_M.1181803258.A.666.html");

        assert_eq!(
            parse_author(&documents),
            ("eggimage".to_owned(), Some("雞蛋非人哉啊....".to_owned()))
        );
    }

    #[test]
    fn test_parse_board() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");

        assert_eq!(parse_board(&documents), BoardName::SoftJob);
    }

    #[test]
    fn test_parse_date() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");
        let article_date = FixedOffset::east(8 * 3600)
            .ymd(2007, 6, 14)
            .and_hms(14, 18, 43);

        assert_eq!(
            parse_date(&documents, &TW_TIME_OFFSET).unwrap(),
            article_date
        );
    }

    #[test]
    fn test_parse_date_with_abnormal_location() {
        let documents = load_document("../tests/Soft_Job_M.1181824048.A.244.html");
        let article_date = FixedOffset::east(8 * 3600)
            .ymd(2007, 6, 14)
            .and_hms(20, 27, 24);

        assert_eq!(
            parse_date(&documents, &TW_TIME_OFFSET).unwrap(),
            article_date
        );
    }

    #[test]
    fn test_parse_replies() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");
        let article_date = FixedOffset::east(8 * 3600)
            .ymd(2007, 6, 14)
            .and_hms(14, 18, 43);

        assert_eq!(parse_replies(&documents, &article_date).len(), 5)
    }

    #[test]
    fn test_parse_replies_with_warning_message() {
        // contains "檔案過大！部分文章無法顯示"
        let documents = load_document("../tests/Gossiping_M.1119222611.A.7A9.html");
        let article_date = FixedOffset::east(8 * 3600)
            .ymd(2005, 6, 20)
            .and_hms(7, 11, 31);

        assert_eq!(parse_replies(&documents, &article_date).len(), 1491)
    }

    #[test]
    fn test_parse_article_without_reply() {
        let documents = load_document("../tests/Soft_Job_M.1181804025.A.7A7.html");
        let article_date = FixedOffset::east(8 * 3600)
            .ymd(2007, 6, 14)
            .and_hms(14, 53, 44);

        assert_eq!(parse_replies(&documents, &article_date).len(), 0)
    }

    #[test]
    fn test_parse_ip() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");
        assert_eq!(parse_ip(&documents), Ipv4Addr::new(125, 232, 236, 105));
    }
}
