use std::{fs::File, io::Write};

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    env_logger::init();

    /*let articles = match fetch_articles_from_website().await {
        Ok(a) => a,
        Err(e) => {
            log::error!("{e}");
            return;
        }
    };

    match save_json(&articles) {
        Ok(_) => (),
        Err(e) => log::error!("{e}"),
    }*/

    let mut articles = match load_json() {
        Ok(a) => a,
        Err(e) => {
            log::error!("{e}");
            return;
        }
    };

    for article in articles.iter_mut() {
        // println!("{}", article.original_title);
        article.cleanup();
        if article.content.is_none() && article.error_message.is_none() {
            article.fill_content().await.unwrap();
        }
        article.parse_content().unwrap();
    }

    match save_json(&articles) {
        Ok(_) => (),
        Err(e) => log::error!("{e}"),
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct NewsArticle {
    original_url: Option<String>,
    original_date: Option<String>,
    original_title: Option<String>,
    original_content_full: Option<String>, // full card content for debugging purposes, potentially including more than necessary depending on article formatting
    original_content: Option<String>, // content with stricter filtering, ideally only including the actual news text

    error_message: Option<String>,

    content: Option<String>,
}

async fn fetch_articles_from_website() -> anyhow::Result<Vec<NewsArticle>> {
    // the URLs apparently don't work without the hash, so we just keep a list of all pages here,
    // instead of just replacing the page number
    let news_urls: Vec<&str> = vec![
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=1&cHash=2232feb5f3e275c761576fa1b601be3d",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=2&cHash=80244d1c490a1eb55c6e54175dacdd5a",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=3&cHash=2acd662b9162b78a9559ad763e84831e",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=4&cHash=28badb79c9fcdf32e1e63d11c90bff92",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=5&cHash=5442a59fcf49f20ac3b32fdd46cb18d4",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=6&cHash=67e1d4b1c758e2729c15b8ccf5784a14",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=7&cHash=2cd37177d211e2f8839322fd8f7d9bbe",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=8&cHash=cd011ec13a8e3d709c500148955c3593",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=9&cHash=1a5406666e639147e2f8f9e4b2b2be58",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=10&cHash=df20ce789940ad61d152bbfcc4e23e13",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=11&cHash=462d0626a8f5e7b98d1a6b09eabbcb7f",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=12&cHash=5492c975ed28c707d502b56447d042a8",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=13&cHash=5cd6a13db867eead43bf648a33fae801",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=14&cHash=734c802f9b6284979a4ed636e6f1a67c",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=15&cHash=53c5108e7147f1cf6445d6ef25e59df8",
        "https://hpi.de/doellner/news/archive.html?oc107385%5Bpage%5D=16&cHash=751446274b7724b6b1ecb2b5aa9287d3",
    ];

    let mut articles = vec![];

    for url in news_urls {
        let html = fetch_html(url).await?;
        let mut overview_articles = parse_overview_html(html.as_str())?;
        articles.append(&mut overview_articles);

        for article in &mut articles {
            article.fill_content().await.unwrap();
        }
    }

    Ok(articles)
}

async fn fetch_html(url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::default();
    let html = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(html)
}

fn parse_overview_html(html: &str) -> anyhow::Result<Vec<NewsArticle>> {
    let document = Html::parse_document(html);
    let card_selector = Selector::parse(".news").unwrap();

    let card_selections = document.select(&card_selector);

    let count = card_selections.clone().count();
    log::info!("found {count} articles");

    let title_selector = Selector::parse(".h2 a").unwrap();
    let date_selector = Selector::parse(".date").unwrap();

    let mut result = vec![];

    for card_element in card_selections {
        let title_element = match card_element.select(&title_selector).next() {
            Some(t) => t,
            None => {
                log::warn!("could not find title, skipping entry");
                continue;
            }
        };

        let title = title_element.inner_html().trim().to_string();
        log::debug!("title: {title}");

        let url = match title_element.attr("href") {
            Some(u) => u.to_string(),
            None => {
                log::warn!("could not find url, skipping entry");
                continue;
            }
        };
        log::debug!("url: {url}");

        let date_element = match card_element.select(&date_selector).next() {
            Some(d) => d,
            None => {
                log::warn!("could not find date, skipping entry");
                continue;
            }
        };

        let date = date_element.inner_html().trim().to_string();
        log::debug!("date: {date}");

        result.push(NewsArticle {
            original_url: Some(url),
            original_date: Some(date),
            original_title: Some(title),
            original_content_full: None,
            original_content: None,
            error_message: None,
            content: None,
        });
    }

    Ok(result)
}

impl NewsArticle {
    async fn fill_content(&mut self) -> anyhow::Result<()> {
        let full_url = format!("https://hpi.de{}", self.original_url.clone().unwrap());
        let html = match fetch_html(full_url.as_str()).await {
            Ok(h) => h,
            Err(e) => {
                log::error!("could not fetch article content: {e}");
                self.error_message = Some(e.to_string());
                return Ok(());
            }
        };

        let document = Html::parse_document(html.as_str());

        let content_selector_full = Selector::parse(".csc-textpic-text").unwrap();
        let content_selector = Selector::parse(".csc-textpic-text > p:nth-last-child(1)").unwrap();
        let content_element_full = match document.select(&content_selector_full).next() {
            Some(c) => {
                let content_full = c.inner_html().trim().to_string();
                self.original_content_full = Some(content_full);
            }
            None => {
                log::warn!("could not find content (full)");
                // return Ok(()); // yeah I know, we should probably return an actual error here
            }
        };
        let content_element = match document.select(&content_selector).next() {
            Some(c) => {
                let content = c.inner_html().trim().to_string();
                self.original_content = Some(content);
            }
            None => {
                log::warn!("could not find content");
                // return Ok(());
            }
        };

        log::debug!("content: {:?}", self.original_content);

        Ok(())
    }

    fn cleanup(&mut self) {
        if self
            .original_content_full
            .as_ref()
            .is_some_and(|s| s.is_empty())
        {
            self.original_content_full = None;
        }
        if self.original_content.as_ref().is_some_and(|s| s.is_empty()) {
            self.original_content = None;
        }
        if self.error_message.as_ref().is_some_and(|s| s.is_empty()) {
            self.error_message = None;
        }
        if self.content.as_ref().is_some_and(|s| s.is_empty()) {
            self.content = None;
        }
    }

    fn parse_content(&mut self) -> anyhow::Result<()> {
        let full_content = match self.original_content_full.clone() {
            Some(s) => s,
            None => return Ok(()), // TODO: actual error/warning?
        };
        let content_fragment = Html::parse_fragment(&full_content);
        let content_selector = Selector::parse("p").unwrap();

        let mut content = "".to_string();

        for paragraph in content_fragment.select(&content_selector) {
            content.push_str(&paragraph.html());
        }

        self.content = Some(content);

        Ok(())
    }
}

fn save_json(articles: &Vec<NewsArticle>) -> anyhow::Result<()> {
    let mut file = File::create("articles.json")?;
    // serde_json::to_writer(file, articles)?;
    let output_string = serde_json::to_string_pretty(articles)?;
    file.write_all(output_string.as_bytes())?;
    Ok(())
}

fn load_json() -> anyhow::Result<Vec<NewsArticle>> {
    let file = File::open("articles.json")?;
    let articles = serde_json::from_reader(file)?;
    Ok(articles)
}
