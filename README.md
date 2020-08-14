# ptt-crawler (ptc)&emsp; [![Crates.io latest version badge]][Crates.io link] [![Docs.rs badge]][Docs.rs link] [![Crates.io download latest badge]][Crates.io link] [![Crates.io license badge]][License file]

[Crates.io latest version badge]: https://img.shields.io/crates/v/ptt-crawler
[Crates.io link]: https://crates.io/crates/ptt-crawler
[Crates.io download latest badge]: https://img.shields.io/crates/d/ptt-crawler
[Crates.io license badge]: https://img.shields.io/crates/l/ptt-crawler
[Docs.rs badge]: https://docs.rs/ptt-crawler/badge.svg
[Docs.rs link]: https://docs.rs/ptt-crawler/
[License file]: https://github.com/cwouyang/ptt-crawler/blob/master/LICENSE

> A crawler for the web version of [PTT](https://www.ptt.cc/index.html), the largest online community in Taiwan.

Yet another PTT crawler but written in Rust. Can be used as binary directly or as crate.

Table of Contents
=================

   * [ptt-crawler (ptc)](#ptt-crawler-ptc----)
   * [Table of Contents](#table-of-contents)
      * [Features](#features)
      * [Getting started](#getting-started)
         * [Installation](#installation)
            * [From crates.io](#from-cratesio)
            * [From the sources](#from-the-sources)
         * [How to use](#how-to-use)
      * [Used as crate](#used-as-crate)
      * [Run unit tests](#run-unit-tests)
      * [Contributing](#contributing)
      * [Links](#links)
      * [Versioning](#versioning)
      * [License](#license)

Created by [gh-md-toc](https://github.com/ekalinin/github-markdown-toc)

## Features

* Single executable without any dependence
* Cross platforms supported
* Crawl single article or multiple articles in one board
* Anti-Anti-Crawler with random user agent and proxy server

## Getting started

### Installation

The binary name for ptt-crawler is `ptc` . 
Currently, no precompiled binary is available.
You need Rust **1.40** or higher and use `cargo` to build ptt-crawler from the sources.

#### From crates.io

``` shell
> cargo install ptt-crawler
```

#### From the sources

``` shell
> git clone https://github.com/cwouyang/ptt-crawler.git
> cd ptt-crawler
> cargo build --release
```

### How to use

* Crawls specific article

``` shell
> ptc url https://www.ptt.cc/bbs/Gossiping/M.1597463395.A.478.html
```

Specify flags user agent `-u` and proxy `-p` used during crawling

``` shell
> ptc -u "user/agent/string" -p "https://some.proxy" url https://www.ptt.cc/bbs/Gossiping/M.1597463395.A.478.html

# pass "random" to use randomly generated user agent
> ptc -u "random" https://www.ptt.cc/bbs/Gossiping/M.1597463395.A.478.html
```

* Crawls articles of board within page range

``` shell
# From page 100 (https://www.ptt.cc/bbs/Gossiping/index100.html) to 200 (https://www.ptt.cc/bbs/Gossiping/index200.html)
> ptc board Gossiping -r 100 200

# From page 1 to latest page
> ptc board Gossiping
```

Use `-l` flag to list supported boards

``` shell
> ptc board Gossiping --list
````

## Used as crate

Add `ptt-crawler` as dependence in `Cargo.toml` file

``` toml
[dependencies]
ptt-crawler = "0.1"
```

See [document](https://docs.rs/ptt-crawler/) for usages.

## Run unit tests

``` shell
> cargo test --all
```

## Contributing

If you'd like to contribute, please fork the repository and use a feature
branch. Pull requests are warmly welcome.

Before submit pull request, make sure 

* [clippy](https://github.com/rust-lang/rust-clippy) was applied
* Commit messages with [Conventional Commits](https://www.conventionalcommits.org/) (See [here](https://github.com/angular/angular/blob/master/CONTRIBUTING.md#-commit-message-format) for detailed format)

## Links

* Project homepage: https://github.com/cwouyang/ptt-crawler/
* Issue tracker: https://github.com/cwouyang/ptt-crawler/issues
* Related projects
  + [ptt-web-crawler](https://github.com/jwlin/ptt-web-crawler): PTT crawler in Python version

  

## Versioning

We use [SemVer](https://semver.org/) for versioning. For the versions available, see the [tags on this repository](https://github.com/cwouyang/ptt-crawler/tags).

## License

Copyright (c) 2020 cwouyang.

This project is licensed under the terms of MIT License. See the [LICENSE]((https://github.com/cwouyang/ptt-crawler/blob/master/LICENSE)) file for details.
