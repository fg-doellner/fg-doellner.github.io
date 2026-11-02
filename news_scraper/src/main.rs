use std::{
    fs::{File, create_dir_all},
    io::Write,
    process::{Command, Stdio},
};

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    env_logger::init();

    // commented out, because we skip actually fetching the articles for now, and instead load the already saved json
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
        article.parse_date();
    }

    match save_json(&articles) {
        Ok(_) => (),
        Err(e) => log::error!("{e}"),
    }

    match export_json_and_pug(&articles) {
        Ok(_) => (),
        Err(e) => log::error!("{e}"),
    }

    html_to_pug(articles[5].content.as_ref().unwrap().as_str()).unwrap();
}

#[derive(Debug, Deserialize, Serialize)]
struct NewsArticle {
    original_url: Option<String>,
    original_date: Option<String>,
    original_title: Option<String>,
    original_content_full: Option<String>, // full card content for debugging purposes, potentially including more than necessary depending on article formatting
    original_content: Option<String>, // content with stricter filtering, ideally only including the actual news text

    error_message: Option<String>,

    date: Option<String>,
    title: Option<String>,
    // `abstract` is not allowed as field name, so we have to manually rename it
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    url: Option<String>,
    detailpage: Option<bool>,

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
            date: None,
            title: None,
            abstract_text: None,
            url: None,
            detailpage: Some(true),
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
        match document.select(&content_selector_full).next() {
            Some(c) => {
                let content_full = c.inner_html().trim().to_string();
                self.original_content_full = Some(content_full);
            }
            None => {
                log::warn!("could not find content (full)");
            }
        };
        match document.select(&content_selector).next() {
            Some(c) => {
                let content = c.inner_html().trim().to_string();
                self.original_content = Some(content);
            }
            None => {
                log::warn!("could not find content");
            }
        };

        // log::debug!("content: {:?}", self.original_content);

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
        self.detailpage = Some(true);
        self.title = self.original_title.clone();

        // todo: put in extra function
        if self.content.is_some() {
            /*let options = html_cleaning::CleaningOptions::builder()
                            .remove_tags(&["script", "style", "span", "a", "strong", "em"])
                            .build();
                        let cleaner = html_cleaning::HtmlCleaner::with_options(options);
                        let doc = html_cleaning::Document::from(self.content.as_ref().unwrap().as_str());
                        cleaner.clean(&doc);

                        let mut abstract_string = doc.select("body")
                                .children()
                                .first() // get first <p>
                                .inner_html() // get content without <p>…</p> around it
                                .to_string();

                        abstract_string = abstract_string.replace("&nbsp;", " ");
            */

            let doc = html_cleaning::dom::parse(self.content.as_ref().unwrap().as_str());
            let selection = doc.select("body").children().first(); // only get first `<p>`
            let mut abstract_string = html_cleaning::dom::text_content(&selection);
            abstract_string = abstract_string.replace(" .", ".");

            self.abstract_text = Some(abstract_string)
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
            let mut html = paragraph.html();

            html = html.replace("&nbsp;", " "); // normalize spaces
            html = html.replace(" </a>", "</a>"); // remove extra whitespace at ends of links

            content.push_str(&html);
        }

        self.content = Some(content);

        Ok(())
    }

    fn parse_date(&mut self) {
        let orig_date = match self.original_date {
            Some(ref d) => d,
            None => {
                log::error!("can't fill in date, since no original_date is available");
                return;
            }
        };

        let segments: Vec<&str> = orig_date.split(".").collect(); // "dd.mm.yyyy" -> ["dd", "mm", "yyyy"]
        if segments.len() != 3 {
            log::error!("failed to split date");
            return;
        }

        let dd = segments[0];
        let mm = segments[1];
        let yyyy = segments[2];

        let iso_date = format!("{yyyy}-{mm}-{dd}");
        log::debug!("{iso_date}");
        self.date = Some(iso_date);
    }

    fn news_key(&self) -> String {
        let mut key = format!(
            "{}-{}",
            self.date.as_ref().unwrap(),
            self.title.as_ref().unwrap()
        )
        .to_lowercase();

        key = sanitize_filename::sanitize(key);
        key = key.replace(" ", "-");
        key = key.replace("@", "_");
        key = key.replace("!", "_");
        key = key.replace("'", "_");
        key = key.replace("\"", "_");

        key
    }
}

fn save_json(articles: &Vec<NewsArticle>) -> anyhow::Result<()> {
    let mut file = File::create("articles.json")?;
    // serde_json::to_writer(file, articles)?;
    let output_string = serde_json::to_string_pretty(articles)?;
    file.write_all(output_string.as_bytes())?;
    Ok(())
}

fn export_json_and_pug(articles: &Vec<NewsArticle>) -> anyhow::Result<()> {
    let out_dir = "export";

    create_dir_all(out_dir)?;

    for article in articles {
        if article.content.is_none() || article.title.is_none() || article.date.is_none() {
            continue;
        }

        let filename_base = article.news_key();

        log::debug!("sanitized: {filename_base}");

        {
            let mut file = File::create(format!("{out_dir}/{filename_base}.json"))?;
            let output_string = serde_json::to_string_pretty(article)?;
            file.write_all(output_string.as_bytes())?;
        }

        {
            let mut file = File::create(format!("{out_dir}/{filename_base}.pug"))?;
            let output_string = article_to_pug(article)?;
            file.write_all(output_string.as_bytes())?;
        }
    }

    Ok(())
}

fn load_json() -> anyhow::Result<Vec<NewsArticle>> {
    let file = File::open("articles.json")?;
    let articles = serde_json::from_reader(file)?;
    Ok(articles)
}

fn html_to_pug(html: &str) -> anyhow::Result<String> {
    let options = html_cleaning::CleaningOptions::builder()
        .remove_tags(&["script", "style", "span"])
        .build();
    let cleaner = html_cleaning::HtmlCleaner::with_options(options);
    let doc = html_cleaning::Document::from(html);
    cleaner.clean(&doc);
    let cleaned_html = doc
        .select("body")
        .inner_html() // get content without <p>…</p> around it
        .to_string();

    let mut cmd = Command::new("html2pug")
        .args(["--fragment", "--double-quotes"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = cmd.stdin.take().expect("failed to spawn child process");

    let input = cleaned_html;

    std::thread::spawn(move || {
        stdin
            .write_all(input.as_bytes())
            .expect("failed to write to stdin");
    });

    let output = cmd.wait_with_output()?;
    let pug = String::from_utf8(output.stdout)?;

    Ok(pug)
}

fn article_to_pug(article: &NewsArticle) -> anyhow::Result<String> {
    let content_pug = html_to_pug(article.content.as_ref().unwrap().as_str())?;

    // pug result is indented with two spaces, but needs for to fit the template
    let mut content_pug_indented = String::new();
    for line in content_pug.lines() {
        content_pug_indented.push_str("  ");
        content_pug_indented.push_str(line);
        content_pug_indented.push_str("\n");
    }

    let article_pug = format!(
        "extends /template/newsentry.pug

block newsvariables
  - var newskey = '{}';

//-
  block date
    | [date]

  block title
    | [title]

block newscontent
  p.card-text
{}",
        article.news_key(),
        content_pug_indented,
    );

    Ok(article_pug.into())
}
