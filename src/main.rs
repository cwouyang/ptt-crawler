extern crate fake_useragent;
extern crate log;
extern crate pretty_env_logger;
extern crate pttcrawler;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use enum_iterator::IntoEnumIterator;
use fake_useragent::UserAgents;
use reqwest::{Client, Proxy};
use structopt::StructOpt;

use pttcrawler::article::BoardName;
use pttcrawler::crawler;

#[derive(StructOpt)]
#[structopt(
    name = "ptt-crawler",
    about = "A crawler for downloading article from web version of PTT",
    version = "0.1.0",
    author = "cwouyang <cwouyang@protonmail.com>"
)]
struct Opt {
    /// Activates debug mode
    #[structopt(short, long)]
    debug: bool,
    /// Outputs results to file in JSON format
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
    /// User agent that crawler should use. Pass "random" to use randomly generated one.
    #[structopt(short, long, parse(from_os_str))]
    user_agent: Option<PathBuf>,
    /// Proxy URL that crawler should pass requests to
    #[structopt(short, long, parse(from_os_str))]
    proxy: Option<PathBuf>,
    /// Timeout in ms for the connect phase of a request
    #[structopt(short, long, default_value = "3000")]
    timeout: u64,

    #[structopt(subcommand)]
    cmd: SubCommand,
}

#[derive(StructOpt)]
enum SubCommand {
    /// Crawls given board with page range
    Board {
        /// Lists available boards
        #[structopt(short = "l", long = "list")]
        show_list: bool,
        /// Board name
        #[structopt(name = "Board", parse(from_os_str))]
        board: PathBuf,
        /// Range of page index. If option is absent, all pages will be processed.
        #[structopt(short, long, max_values(2))]
        range: Option<Vec<u32>>,
    },
    /// Crawls given URL of article directly
    Url {
        /// URL of article to crawl
        #[structopt(name = "URL", parse(from_os_str))]
        url: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    if opt.debug {
        std::env::set_var("RUST_LOG", "pttcrawler=debug");
        pretty_env_logger::init();
    }

    let mut user_agent: Option<String> = None;
    if let Some(ua) = opt.user_agent {
        let ua_string = ua.into_os_string().into_string().unwrap();
        if ua_string == "random" {
            let user_agents = UserAgents::new();
            user_agent = Some(user_agents.random().to_owned());
        } else {
            user_agent = Some(ua_string);
        }
    }

    let mut proxies: Option<Vec<Proxy>> = None;
    if let Some(proxy_string) = opt.proxy {
        let proxy = reqwest::Proxy::https(&proxy_string.into_os_string().into_string().unwrap())
            .unwrap_or_else(|e| {
                eprintln!("Error: Invalid format of proxy\n{:#?}", e);
                process::exit(1);
            });
        proxies = Some(vec![proxy])
    }

    let json_output: String;
    match opt.cmd {
        SubCommand::Url { url } => {
            let url_string = url.into_os_string().into_string().unwrap();

            println!("Start crawling URL \"{}\"", url_string);
            let client = create_client(user_agent, proxies, opt.timeout).await;
            json_output = match crawler::crawl_url(&client, &url_string, None).await {
                Ok(article) => serde_json::to_string_pretty(&article).unwrap(),
                Err(e) => {
                    eprintln!("Error: Failed to crawl with error\n{:#?}", e);
                    process::exit(1)
                }
            };
        }
        SubCommand::Board {
            show_list,
            board,
            range,
        } => {
            if show_list {
                for board in BoardName::into_enum_iter() {
                    println!("{}", board.to_string());
                }
                process::exit(0);
            }

            let board_string = board.into_os_string().into_string().unwrap();
            let board = board_string.parse::<BoardName>().unwrap_or_else(|_| {
                eprintln!(
                    "Error: Invalid board name \"{}\". Use --list to see available options",
                    board_string
                );
                process::exit(1);
            });
            let client = create_client(user_agent, proxies, opt.timeout).await;
            let page_count = crawler::crawl_page_count(&client, &board)
                .await
                .unwrap_or(0);
            let range = adjust_board_range(page_count, range)
                .await
                .unwrap_or_else(|_| {
                    eprintln!(
                        "Error: Invalid page range. Should between 1 and {}",
                        page_count
                    );
                    process::exit(1);
                });

            println!(
                "Start crawling board \"{}\" from page {} to {}",
                board,
                range.start(),
                range.end()
            );
            json_output = match crawler::crawl_page_articles(&client, &board, &range).await {
                Ok(articles) => serde_json::to_string_pretty(&articles).unwrap(),
                Err(e) => {
                    eprintln!("Error: Failed to crawl with error\n{:#?}", e);
                    process::exit(1);
                }
            };
        }
    }

    if let Some(output) = opt.output {
        let mut file = File::create(&output).unwrap_or_else(|_| {
            let alt_output = env::current_dir()
                .unwrap()
                .join("result.json")
                .into_os_string()
                .into_string()
                .unwrap();
            eprintln!(
                "Error: Failed to create file at {}, change to {}",
                output.into_os_string().into_string().unwrap(),
                alt_output
            );
            File::create(alt_output).unwrap()
        });
        file.write_all(json_output.as_bytes()).unwrap_or_else(|e| {
            eprintln!("Error: Failed to write results with error\n{:#?}", e);
            process::exit(1)
        });
    } else {
        println!("Results in JSON format:\n{}", json_output);
    }
}

async fn create_client(
    user_agent: Option<String>,
    proxies: Option<Vec<Proxy>>,
    connect_timeout: u64,
) -> Client {
    match crawler::create_client(
        user_agent,
        proxies,
        Some(Duration::from_millis(connect_timeout)),
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error: Failed to create client\n({:#?})", e);
            process::exit(1);
        }
    }
}

async fn adjust_board_range(
    page_count: u32,
    range: Option<Vec<u32>>,
) -> Result<RangeInclusive<u32>, ()> {
    match range {
        Some(mut r) => {
            if r.len() == 1 {
                r.push(page_count);
            }
            // make sure the range is increasing
            if r[0] > r[1] {
                r.swap(0, 1);
            }
            if r[1] > page_count {
                return Err(());
            }
            Ok(r[0]..=r[1])
        }
        None => Ok(1..=page_count),
    }
}
