use std::net::Ipv4Addr;

use chrono::{offset::FixedOffset, offset::LocalResult, prelude::*, DateTime};
use regex::Regex;
use select::predicate::{Attr, Class, Name, Predicate};
use select::{document::Document, node::Node};

use crate::article::{Article, BoardName, Meta, Reply, ReplyCount, ReplyType};

lazy_static! {
    static ref TW_TIME_OFFSET: FixedOffset = FixedOffset::east(8 * 3600);
}

/// Error represents the errors which might occur when parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    DeletedArticle,
    InvalidFormat,
    FieldNotFound(String),
}

pub fn parse(document: &Document) -> Result<Article, Error> {
    if !is_article_exist(&document) {
        warn!("article deleted");
        return Err(Error::DeletedArticle);
    }

    let meta = parse_meta(&document)?;
    let content = parse_content(&document)?;
    let replies = parse_replies(&document, meta.date);

    let reply_count = ReplyCount {
        push: replies
            .iter()
            .filter(|r| r.reply_type == ReplyType::Push)
            .count() as i16,
        neutral: replies
            .iter()
            .filter(|r| r.reply_type == ReplyType::Neutral)
            .count() as i16,
        boo: replies
            .iter()
            .filter(|r| r.reply_type == ReplyType::Boo)
            .count() as i16,
    };
    Ok(Article {
        meta,
        content,
        reply_count,
        replies,
    })
}

fn is_article_exist(document: &Document) -> bool {
    !document
        .find(Class("bbs-content"))
        .any(|n: Node| n.text().contains("404 - Not Found."))
}

fn parse_meta(document: &Document) -> Result<Meta, Error> {
    let id = parse_id(document);
    let (category, title) = match parse_title(document) {
        Ok((Some(category), title)) => (category, title),
        Ok((None, title)) => ("".to_owned(), title),
        Err(e) => return Err(e),
    };
    let (author_id, author_name) = parse_author(document)?;
    let board = parse_board(document)?;
    let date = parse_date(document).ok();
    let ip = parse_ip(document).ok();

    Ok(Meta {
        id,
        category,
        title,
        author_id,
        author_name,
        board,
        date,
        ip,
    })
}

fn parse_id(document: &Document) -> String {
    let url = document
        .find(Name("link").and(Attr("rel", "canonical")))
        .next()
        .unwrap()
        .attr("href")
        .unwrap();
    let split_url = url.split('/').collect::<Vec<_>>();
    let mut id = split_url.last().unwrap().to_owned();
    let html_extension_index: usize = id.find(".html").unwrap();
    id = &id[..html_extension_index];
    id.to_owned()
}

fn parse_title(document: &Document) -> Result<(Option<String>, String), Error> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"((\[|［)(?P<category>\w+)+(\]|］)\s*)?(?P<title>.+)").unwrap();
    }

    let original_title = match document
        .find(Name("meta").and(Attr("property", "og:title")))
        .next()
    {
        Some(n) => n.attr("content").unwrap().to_owned(),
        None => {
            let title_node = document.find(Name("span")).find(|n| {
                let text = n.text();
                text.trim().eq("標題")
            });
            if let Some(n) = title_node {
                n.next().unwrap().text()
            } else {
                let main_content = get_main_content(document);
                match main_content.find("標題:") {
                    Some(mut title_start_index) => {
                        let title = main_content[title_start_index..].to_owned();
                        let title_colon_index = title.find(':').unwrap();
                        let title_end_index = title.find('\n').unwrap();
                        title_start_index = title_colon_index + 1;
                        title[title_start_index..title_end_index].to_owned()
                    }
                    None => {
                        error!("Title field not found");
                        return Err(Error::FieldNotFound("title".to_owned()));
                    }
                }
            }
        }
    };
    let trim_title = original_title.trim();
    Ok(match RE.captures(trim_title) {
        Some(cap) => (
            match cap.name("category") {
                Some(m) => Some(m.as_str().to_owned()),
                None => None,
            },
            cap["title"].to_owned(),
        ),
        None => (None, trim_title.to_owned()),
    })
}

fn parse_author(document: &Document) -> Result<(String, Option<String>), Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<id>\w+)\s\((?P<name>.+)\)").unwrap();
    }

    let author = match document
        .find(Name("span").and(Class("article-meta-value")))
        .next()
    {
        Some(n) => n.text(),
        None => {
            let author_node = document.find(Name("span")).find(|n| {
                let text = n.text();
                text.trim().eq("作者")
            });
            if let Some(n) = author_node {
                n.next().unwrap().text()
            } else {
                let main_content = get_main_content(document);
                match main_content.find("作者:") {
                    Some(mut author_start_index) => {
                        let author = main_content[author_start_index..].to_owned();
                        let author_colon_index = author.find(':').unwrap();
                        let author_end_index = author.find('\n').unwrap();
                        author_start_index = author_colon_index + 1;
                        author[author_start_index..author_end_index].to_owned()
                    }
                    None => {
                        error!("Author field not found");
                        return Err(Error::FieldNotFound("author".to_owned()));
                    }
                }
            }
        }
    };
    let trim_author = author.trim();
    match RE.captures(trim_author) {
        Some(cap) => Ok((cap["id"].to_owned(), Some(cap["name"].to_owned()))),
        None => Ok((trim_author.to_owned(), None)),
    }
}

fn parse_board(document: &Document) -> Result<BoardName, Error> {
    let board = match document
        .find(Name("span").and(Class("article-meta-value")))
        .nth(1)
    {
        Some(n) => n.text(),
        None => {
            let board_node = document.find(Name("span")).find(|n| {
                let text = n.text();
                text.trim().eq("看板")
            });
            if board_node.is_none() {
                error!("Board field not found");
                return Err(Error::FieldNotFound("board".to_owned()));
            }
            board_node.unwrap().next().unwrap().text()
        }
    };
    Ok(board.parse::<BoardName>().unwrap_or(BoardName::Unknown))
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
                None => {
                    error!("Date field not found");
                    return Err(Error::FieldNotFound("date".to_owned()));
                }
            }
        }
    };

    parse_date_from_str(&time_str, &DATE_FORMAT)
}

fn parse_date_from_str(date_str: &str, format: &str) -> Result<DateTime<FixedOffset>, Error> {
    match NaiveDateTime::parse_from_str(date_str, format) {
        Ok(date) => match TW_TIME_OFFSET.from_local_datetime(&date) {
            LocalResult::Single(offset_date) => Ok(offset_date),
            e => {
                error!(
                    "Failed to parse date {:?} from format {:?}\n{:?}",
                    date_str, format, e
                );
                Err(Error::InvalidFormat)
            }
        },
        Err(e) => {
            error!(
                "Failed to parse date {:?} from format {:?}\n{:?}",
                date_str, format, e
            );
            Err(Error::InvalidFormat)
        }
    }
}

fn parse_ip(document: &Document) -> Result<Ipv4Addr, Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<ip>\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})").unwrap();
    }

    let str_contain_ip = match document
        .find(Name("span").and(Class("f2")))
        .map(|n| n.text())
        .find(|s| !s.contains("編輯") && (s.contains("來自:") || s.contains("From:")))
    {
        Some(ip) => ip,
        None => {
            let main_content = get_main_content(document);
            let sub_content_start_index = main_content
                .find("來自:")
                .unwrap_or_else(|| main_content.find("From:").unwrap_or_default());
            main_content[sub_content_start_index..].to_owned()
        }
    };
    match RE.captures(&str_contain_ip) {
        Some(cap) => {
            let ip = &cap["ip"];
            ip.parse::<Ipv4Addr>().map_err(|_| {
                error!("Invalid IP {}", ip);
                Error::FieldNotFound("ip".to_owned())
            })
        }
        None => {
            error!("IP field not found");
            Err(Error::FieldNotFound("ip".to_owned()))
        }
    }
}

fn get_main_content(document: &Document) -> String {
    document
        .find(Name("div").and(Attr("id", "main-content")))
        .next()
        .unwrap()
        .text()
}

fn parse_content(document: &Document) -> Result<String, Error> {
    let main_content = get_main_content(document);
    let content_start_index = match main_content.find('\n') {
        Some(start_index) => start_index,
        None => {
            error!("Failed to find start of content");
            return Err(Error::InvalidFormat);
        }
    };
    let content_end_index = match main_content[(content_start_index + 1)..].find("\n※") {
        Some(end_index) => end_index + content_start_index + 1,
        None => {
            error!("Failed to find end of content");
            return Err(Error::InvalidFormat);
        }
    };
    let content = &main_content[content_start_index..content_end_index];
    Ok(content.trim().to_owned())
}

fn parse_replies(document: &Document, article_time: Option<DateTime<FixedOffset>>) -> Vec<Reply> {
    document
        .find(Name("div").and(Class("push")))
        .flat_map(|n| parse_reply(&n, article_time))
        .collect::<Vec<Reply>>()
}

fn parse_reply(node: &Node, article_time: Option<DateTime<FixedOffset>>) -> Result<Reply, Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"(?P<ip>\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})?\s?(?P<month>\d{2})/(?P<day>\d{2})(\s*(?P<hour>\d{2}):(?P<min>\d{2}))?"
        )
        .unwrap();
    }

    if node.text() == "檔案過大！部分文章無法顯示" {
        warn!("Invalid format of reply {:?}", node.text());
        return Err(Error::InvalidFormat);
    }

    let reply_type = node
        .find(Name("span").and(Class("push-tag")))
        .next()
        .unwrap()
        .text()
        .trim()
        .parse::<ReplyType>()
        .unwrap();
    let author_id = node
        .find(Name("span").and(Class("push-userid")))
        .next()
        .unwrap()
        .text();
    let mut content = node
        .find(Name("span").and(Class("push-content")))
        .next()
        .unwrap()
        .text()
        .trim_start_matches(|c| (c == ':' || c == ' '))
        .trim()
        .to_owned();
    let mut ip_and_time = node
        .find(Name("span").and(Class("push-ipdatetime")))
        .next()
        .unwrap()
        .text();

    ip_and_time = ip_and_time.trim().to_owned();
    let ip_and_time_parser = |cap: regex::Captures| {
        let ip = cap
            .name("ip")
            .map(|m| m.as_str().parse::<Ipv4Addr>().unwrap());
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
        (ip, month, day, hour, min)
    };
    let (ip, month, day, hour, min) = match RE.captures(&ip_and_time) {
        Some(cap) => ip_and_time_parser(cap),
        None => {
            warn!(
                "IP and date of reply \"{:?}\" were not found, try find them in content",
                node.text()
            );
            match RE.captures(&content) {
                Some(cap) => {
                    let (ip, month, day, hour, min) = ip_and_time_parser(cap);
                    // Remove IP and date from content
                    if let Some(ip) = ip {
                        let ip_start_index = content.find(&ip.to_string()).unwrap();
                        content = content[..ip_start_index].trim().to_owned();
                    }
                    (ip, month, day, hour, min)
                }
                None => {
                    warn!("Invalid format of reply {:?}", node.text());
                    return Err(Error::InvalidFormat);
                }
            }
        }
    };

    let date = article_time.and_then(|t| {
        let mut year = t.year();
        if month == 2 && day == 29 {
            while !is_leap_year(year) {
                year += 1;
            }
        }

        match TW_TIME_OFFSET
            .ymd_opt(year, month, day)
            .and_hms_opt(hour, min, 0)
        {
            LocalResult::Single(date) => Some(date),
            _ => None,
        }
    });

    Ok(Reply {
        author_id,
        reply_type,
        ip,
        date,
        content,
    })
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0) && (year % 100 != 0 || year % 400 == 0)
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
                Error::DeletedArticle => true,
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
            parse_title(&documents).unwrap(),
            (Some("公告".to_owned()), "Soft_Job 板試閱".to_owned())
        );
    }

    #[test]
    fn test_parse_title_without_category() {
        let documents = load_document("../tests/Soft_Job_M.1181803258.A.666.html");
        assert_eq!(
            parse_title(&documents).unwrap(),
            (None, "搶頭香".to_owned())
        );
    }

    #[test]
    fn test_parse_title_not_in_html_meta() {
        let documents = load_document("../tests/Gossiping_M.1123769450.A.A1A.html");
        assert_eq!(
            parse_title(&documents).unwrap(),
            (
                Some("名人".to_owned()),
                "有沒有人有希特勒的八卦阿".to_owned()
            )
        );
    }

    #[test]
    fn test_parse_title_within_content() {
        let documents = load_document("../tests/Gossiping_M.1173456473.A.F4F.html");
        assert_eq!(
            parse_title(&documents).unwrap(),
            (None, "Re: 有沒有俄國人的八卦？".to_owned())
        );
    }

    #[test]
    fn test_parse_author() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");

        assert_eq!(
            parse_author(&documents).unwrap(),
            ("Junchoon".to_owned(), Some("裘髯客".to_owned()))
        );
    }

    #[test]
    fn test_parse_author_with_non_chinese() {
        let documents = load_document("../tests/Soft_Job_M.1181803258.A.666.html");

        assert_eq!(
            parse_author(&documents).unwrap(),
            ("eggimage".to_owned(), Some("雞蛋非人哉啊....".to_owned()))
        );
    }

    #[test]
    fn test_parse_author_not_in_html_meta() {
        let documents = load_document("../tests/Gossiping_M.1123769450.A.A1A.html");

        assert_eq!(
            parse_author(&documents).unwrap(),
            ("MOTHERGOOSE".to_owned(), None)
        );
    }

    #[test]
    fn test_parse_author_within_content() {
        let documents = load_document("../tests/Gossiping_M.1173456473.A.F4F.html");
        assert_eq!(
            parse_author(&documents).unwrap(),
            ("julysecond".to_owned(), Some("還是台灣好".to_owned()))
        );
    }

    #[test]
    fn test_parse_board() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");

        assert_eq!(parse_board(&documents).unwrap(), BoardName::SoftJob);
    }

    #[test]
    fn test_parse_board_not_in_html_meta() {
        let documents = load_document("../tests/Gossiping_M.1123769450.A.A1A.html");

        assert_eq!(parse_board(&documents).unwrap(), BoardName::Gossiping);
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
    fn test_parse_date_within_content() {
        let documents = load_document("../tests/Gossiping_M.1173456473.A.F4F.html");
        let article_date = FixedOffset::east(8 * 3600)
            .ymd(2007, 3, 10)
            .and_hms(00, 07, 48);

        assert_eq!(parse_date(&documents).unwrap(), article_date);
    }

    #[test]
    fn test_parse_replies() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");
        let article_date = Some(
            FixedOffset::east(8 * 3600)
                .ymd(2007, 6, 14)
                .and_hms(14, 18, 43),
        );

        assert_eq!(parse_replies(&documents, article_date).len(), 5)
    }

    #[test]
    fn test_parse_replies_with_warning_message() {
        // contains "檔案過大！部分文章無法顯示"
        let documents = load_document("../tests/Gossiping_M.1119222611.A.7A9.html");
        let article_date = Some(
            FixedOffset::east(8 * 3600)
                .ymd(2005, 6, 20)
                .and_hms(7, 11, 31),
        );

        assert_eq!(parse_replies(&documents, article_date).len(), 1491)
    }

    #[test]
    fn test_parse_replies_with_invalid_date() {
        // contains "03/32"
        let documents = load_document("../tests/WomenTalk_M.1143885175.A.C8D.html");
        let article_date = Some(
            FixedOffset::east(8 * 3600)
                .ymd(2006, 4, 1)
                .and_hms(18, 9, 31),
        );

        let replies = parse_replies(&documents, article_date);

        for i in 0..=5 {
            assert_eq!(replies[i].date, None);
        }
    }

    #[test]
    fn test_parse_article_without_reply() {
        let documents = load_document("../tests/Soft_Job_M.1181804025.A.7A7.html");
        let article_date = Some(
            FixedOffset::east(8 * 3600)
                .ymd(2007, 6, 14)
                .and_hms(14, 53, 44),
        );

        assert_eq!(parse_replies(&documents, article_date).len(), 0)
    }

    #[test]
    fn test_parse_ip() {
        let documents = load_document("../tests/Soft_Job_M.1181801925.A.86E.html");
        assert_eq!(
            parse_ip(&documents).unwrap(),
            Ipv4Addr::new(125, 232, 236, 105)
        );
    }

    #[test]
    fn test_parse_ip2() {
        let documents = load_document("../tests/Gossiping_M.1119222660.A.94E.html");
        assert_eq!(
            parse_ip(&documents).unwrap(),
            Ipv4Addr::new(138, 130, 212, 179)
        );
    }

    #[test]
    fn test_parse_ip3() {
        let documents = load_document("../tests/Gossiping_M.1175469904.A.05B.html");
        assert_eq!(
            parse_ip(&documents).unwrap(),
            Ipv4Addr::new(140, 118, 229, 94)
        );
    }

    #[test]
    fn test_parse_invalid_ip() {
        let documents = load_document("../tests/Soft_Job_M.1519661420.A.098.html");
        assert_eq!(
            parse_ip(&documents),
            Err(Error::FieldNotFound("ip".to_owned()))
        );
    }

    #[test]
    fn test_parse_malformed_content() {
        let documents = load_document("../tests/Gossiping_M.1519661420.A.098.html");
        assert_eq!(parse_content(&documents), Err(Error::InvalidFormat));
    }
}
