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
    let date = parse_date(document).unwrap();
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

fn parse_date(document: &Document) -> Result<DateTime<FixedOffset>, Error> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"(?P<date>\w{3} \w{3} \d{2} \d{2}:\d{2}:\d{2} \d{4})").unwrap();
        static ref DATE_FORMAT: &'static str = "%a %b  %e %H:%M:%S %Y";
    }

    let time_str = match document
        .find(Name("span").and(Class("article-meta-value")))
        .nth(3)
    {
        Some(node) => node.text(),
        None => {
            let main_content = get_main_content(document);
            match RE.captures(&main_content) {
                Some(cap) => cap["date"].to_owned(),
                None => return Err(Error::FieldNotFound),
            }
        }
    };

    parse_date_from_str(&time_str, &DATE_FORMAT)
}

fn parse_date_from_str(date_str: &str, format: &str) -> Result<DateTime<FixedOffset>, Error> {
    match NaiveDateTime::parse_from_str(date_str, format) {
        Ok(date) => match TW_TIME_OFFSET.from_local_datetime(&date) {
            LocalResult::Single(offset_date) => Ok(offset_date),
            _ => Err(Error::InvalidFormat),
        },
        Err(e) => {
            error!("{:?}", e);
            Err(Error::InvalidFormat)
        }
    }
}

fn parse_ip(document: &Document) -> Ipv4Addr {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<ip>\d{1,3}.\d{1,3}.\d{1,3}.\d{1,3})").unwrap();
    }

    let str_contain_ip = match document
        .find(Name("span").and(Class("f2")))
        .map(|n| n.text())
        .find(|s| (s.contains("來自:") || s.contains("From:")))
    {
        Some(ip) => ip.to_owned(),
        None => {
            let main_content = get_main_content(document);
            let sub_content_start_index = main_content
                .find("來自:")
                .unwrap_or_else(|| main_content.find("From:").unwrap_or_default());
            main_content[sub_content_start_index..].to_owned()
        }
    };
    match RE.captures(&str_contain_ip) {
        Some(cap) => cap["ip"].parse::<Ipv4Addr>().unwrap(),
        None => Ipv4Addr::new(0, 0, 0, 0),
    }
}

fn get_main_content(document: &Document) -> String {
    document
        .find(Name("div").and(Attr("id", "main-content")))
        .nth(0)
        .unwrap()
        .text()
}

fn parse_content(document: &Document) -> String {
    let main_content = get_main_content(document);
    let content_start_index = main_content.find('\n').unwrap();
    let content_end_index = main_content.find("\n※").unwrap();
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
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"(?P<ip>\d{1,3}.\d{1,3}.\d{1,3}.\d{1,3})?\s?(?P<month>\d{2})/(?P<day>\d{2})(\s*(?P<hour>\d{2}):(?P<min>\d{2}))?"
        )
        .unwrap();
    }

    if node.text() == "檔案過大！部分文章無法顯示" {
        error!("Invalid format of reply {:?}", node.text());
        return Err(Error::InvalidFormat);
    }

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
    let mut ip_and_time = node
        .find(Name("span").and(Class("push-ipdatetime")))
        .nth(0)
        .unwrap()
        .text();
    ip_and_time = ip_and_time.trim().to_owned();
    let (ip, month, day, hour, min) = match RE.captures(&ip_and_time) {
        Some(cap) => {
            let ip_str = match cap.name("ip") {
                Some(m) => m.as_str(),
                None => "0.0.0.0",
            };
            let ip = ip_str.parse::<Ipv4Addr>().unwrap();
            let month = cap["month"].parse::<u32>().unwrap();
            let day = cap["day"].parse::<u32>().unwrap();
            let hour: u32 = match cap.name("hour") {
                Some(m) => m.as_str().parse::<u32>().unwrap(),
                None => 0,
            };
            let min: u32 = match cap.name("min") {
                Some(m) => m.as_str().parse::<u32>().unwrap(),
                None => 0,
            };
            (Some(ip), month, day, hour, min)
        }
        None => {
            error!("Invalid format of reply {:?}", node.text());
            return Err(Error::InvalidFormat);
        }
    };

    let mut year = article_year;
    if month == 2 && day == 29 {
        while !is_leap_year(year) {
            year += 1;
        }
    }
    let date = TW_TIME_OFFSET.ymd(year, month, day).and_hms(hour, min, 0);

    Ok(Reply {
        author_id,
        reply_type,
        content,
        ip,
        date,
    })
}

fn is_leap_year(year: i32) -> bool {
    return (year % 4 == 0) && (year % 100 != 0 || year % 400 == 0);
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
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

        assert_eq!(parse_date(&documents).unwrap(), article_date);
    }

    #[test]
    fn test_parse_date_with_abnormal_location() {
        let documents = load_document("../tests/Soft_Job_M.1181824048.A.244.html");
        let article_date = FixedOffset::east(8 * 3600)
            .ymd(2007, 6, 14)
            .and_hms(20, 27, 24);

        assert_eq!(parse_date(&documents).unwrap(), article_date);
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

    #[test]
    fn test_parse_ip2() {
        let documents = load_document("../tests/Gossiping_M.1119222660.A.94E.html");
        assert_eq!(parse_ip(&documents), Ipv4Addr::new(138, 130, 212, 179));
    }
}
