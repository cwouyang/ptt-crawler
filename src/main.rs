extern crate log;
extern crate pretty_env_logger;
extern crate pttcrawler;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Range;
use std::path::PathBuf;
use std::process;

use enum_iterator::IntoEnumIterator;
use reqwest::Client;
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

fn main() {
    let opt = Opt::from_args();

    if opt.debug {
        std::env::set_var("RUST_LOG", "pttcrawler=debug");
        pretty_env_logger::init();
    }

    let json_output: String;
    match opt.cmd {
        SubCommand::Url { url } => {
            let url_string = url.into_os_string().into_string().unwrap();

            println!("Start crawling URL \"{}\"", url_string);
            let client = create_client();
            json_output = crawler::crawl_url(&client, &url_string)
                .map(|article| serde_json::to_string_pretty(&article).unwrap())
                .unwrap_or_else(|e| {
                    eprintln!("Error: Failed to crawl with error {:?}", e);
                    process::exit(1)
                });
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
            let client = create_client();
            let range = get_board_range(&client, &board, range);

            println!(
                "Start crawling board \"{}\" from page {} to {}",
                board, range.start, range.end
            );
            json_output = crawler::crawl_pages(&client, board, range)
                .map(|articles| serde_json::to_string_pretty(&articles).unwrap())
                .unwrap_or_else(|e| {
                    eprintln!("Error: Failed to crawl with error {:?}", e);
                    process::exit(1);
                });
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
            eprintln!("Error: Failed to write results with error {:?}", e);
            process::exit(1)
        });
    } else {
        println!("Results in JSON format:\n{}", json_output);
    }
}

fn create_client() -> Client {
    let client = match crawler::create_client() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error: Failed to create client ({:#?})", e);
            process::exit(1);
        }
    };
    client
}

fn get_board_range(client: &Client, board: &BoardName, range: Option<Vec<u32>>) -> Range<u32> {
    match range {
        Some(mut r) => {
            if r.len() == 1 {
                match crawler::crawl_page_count(&client, &board) {
                    Ok(page_count) => r.push(page_count + 1),
                    Err(_) => r.push(r[0] + 1),
                };
            }
            // make sure the range is increasing
            if r[0] > r[1] {
                r.swap(0, 1);
            }
            r[0]..r[1]
        }
        None => match crawler::crawl_page_count(&client, &board) {
            Ok(page_count) => 1..(page_count + 1),
            Err(_) => 1..2,
        },
    }
}
