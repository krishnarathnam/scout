use crate::display;
use crate::income_statement;
use anyhow::Ok;
use anyhow::Result;
use csv::Reader;
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

pub async fn get_news(client: &reqwest::Client, symbol: &String) -> Result<()> {
    let yf_client = yfinance_rs::YfClient::default();
    let ticker = yfinance_rs::Ticker::new(&yf_client, symbol);

    let news = ticker.news().await?;
    for article in news {
        if let Some(link) = article.link {
            let title = article.title;
            let response = client.get(&link).send().await?;

            if !response.status().is_success() {
                println!(
                    "Could not fetch data for {title}: HTTP {}",
                    response.status(),
                );
            } else {
                println!("fetched data for {title} - {link}");
            }

            let _body = response.text().await?;
            //println!("###{title}\n {body}");
        }
    }

    Ok(())
}
