use crate::display;
use crate::income_statement;
use anyhow::Ok;
use anyhow::Result;
use csv::Reader;
use scraper::Html;
use scraper::Selector;
use std::fmt::Write;
use std::fs::File;
use strsim::jaro_winkler;

pub fn find_ticker(company: &str) -> Option<String> {
    let file = File::open("data/nse.csv").ok()?;
    let mut rdr = Reader::from_reader(file);

    let mut best = None;
    let mut best_score = 0.0;

    for row in rdr.records() {
        let r = row.ok()?;
        let symbol = &r[0];
        let name = &r[1];

        let score = jaro_winkler(&company.to_lowercase(), &name.to_lowercase());

        if score > best_score {
            best_score = score;
            best = Some(symbol.to_string());
        }
    }

    if best_score > 0.80 {
        best.map(|s| format!("{}.NS", s))
    } else {
        None
    }
}

pub async fn get_financials(
    symbol: &String,
    client: &reqwest::Client,
    finance: &str,
) -> Result<String> {
    let mut link: String = String::new();
    let mut title: String = String::new();
    match finance {
        "income_statement" => {
            link = format!("https://finance.yahoo.com/quote/{}/financials", symbol);
            title = format!("Income Statement – {}", symbol);
        }
        "balance_sheet" => {
            link = format!("https://finance.yahoo.com/quote/{}/balance-sheet/", symbol);
            title = format!("Balance Sheet – {}", symbol);
        }
        "cash_flow" => {
            link = format!("https://finance.yahoo.com/quote/{}/cash-flow/", symbol);
            title = format!("Cash Flow – {}", symbol);
        }

        _ => {
            println!("Wrong finance");
        }
    }

    let response = client.get(&link).send().await?;

    if !response.status().is_success() {
        println!("Could not fetch data: HTTP {}", response.status());
        return Err(anyhow::anyhow!("Request failed: {}", response.status()));
    }

    let body = response.text().await?;

    if let Some((headers, rows)) = income_statement::scrape_financials_table(&body) {
        let period_headers: Vec<String> = headers.into_iter().skip(1).collect();
        println!("- Read {title}");
        return Ok(display::print_scraped_table(
            &title.as_str(),
            &period_headers,
            &rows,
        ));
    }

    Err(anyhow::anyhow!(
        "Could not parse financial data (Yahoo may have changed their format)"
    ))
}

pub async fn get_news(client: &reqwest::Client, symbol: &String) -> Result<String> {
    let yf_client = yfinance_rs::YfClient::default();
    let ticker = yfinance_rs::Ticker::new(&yf_client, symbol);

    let news = ticker.news().await?;
    let mut news_combined = String::new();

    for (idx, article) in news.into_iter().enumerate() {
        if let Some(link) = article.link {
            let title = article.title;

            writeln!(&mut news_combined, "========== Article {} ==========", idx + 1).ok();
            writeln!(&mut news_combined, "Title: {}", title).ok();
            writeln!(&mut news_combined, "Link:  {}", link).ok();
            writeln!(&mut news_combined).ok();

            let response = client.get(&link).send().await?;
            if !response.status().is_success() {
                writeln!(
                    &mut news_combined,
                    "[ERROR] Could not fetch article body: HTTP {}",
                    response.status()
                )
                .ok();
                writeln!(&mut news_combined).ok();
                continue;
            } else {
                println!("fetched data for {title} - {link}");
            }

            let body_html = response.text().await?;
            let document = Html::parse_document(&body_html);

            let container_selector = Selector::parse("div.article.yf-1qeh9w1").unwrap();
            let text_selector =
                Selector::parse("p, h1, h2, h3, h4, h5, h6, li, blockquote").unwrap();

            let mut article_text = String::new();

            let is_boilerplate = |t: &str| {
                let l = t.to_lowercase();
                let stop_markers = [
                    "go to accessibility shortcuts",
                    "share",
                    "comments",
                    "read more",
                    "additional sources",
                    "edited by",
                    "the big question",
                ];
                stop_markers.iter().any(|m| l.contains(m))
            };

            if let Some(container) = document.select(&container_selector).next() {
                for element in container.select(&text_selector) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    let text = text.trim();

                    if text.len() < 40 {
                        continue;
                    }
                    if is_boilerplate(text) {
                        break;
                    }

                    if !article_text.is_empty() {
                        article_text.push_str("\n\n");
                    }
                    article_text.push_str(text);
                }
            } else {
                let fallback_selector = Selector::parse(
                    "article, main, p, h1, h2, h3, h4, h5, h6, li, blockquote",
                )
                .unwrap();

                for element in document.select(&fallback_selector) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    let text = text.trim();

                    if text.len() < 40 {
                        continue;
                    }
                    if is_boilerplate(text) {
                        break;
                    }

                    if !article_text.is_empty() {
                        article_text.push_str("\n\n");
                    }
                    article_text.push_str(text);
                }
            }

            let article_text = article_text.trim();
            if !article_text.is_empty() {
                writeln!(&mut news_combined, "{}\n", article_text).ok();
            }
        }
    }

    Ok(news_combined)
}
