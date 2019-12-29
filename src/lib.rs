extern crate chrono;
extern crate strum;
#[macro_use]
extern crate strum_macros;
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
#[macro_use]
extern crate load_file;
extern crate regex;
extern crate reqwest;
extern crate select;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate url;

pub mod article;
pub mod crawler;
mod parser;
