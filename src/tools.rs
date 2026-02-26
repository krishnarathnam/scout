use anyhow::Result;
use csv::Reader;
use std::fs::File;
use strsim::jaro_winkler;
use yfinance_rs::{Interval, Range, Ticker, YfClient};

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

pub async fn get_ticker_info(symbol: &str) -> Result<()> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, symbol);

    let price_target = ticker.analyst_price_target(None).await?;
    println!("{:?}", price_target);

    let rec_sum = ticker.recommendations_summary().await?;
    println!("{:?}", rec_sum);

    let history = ticker
        .history(Some(Range::M6), Some(Interval::D1), false)
        .await?;

    if let Some(last_bar) = history.last() {
        println!(
            "Last closing price: {:.2} on timestamp {}",
            yfinance_rs::core::conversions::money_to_f64(&last_bar.close),
            last_bar.ts
        );
    }

    let news = ticker.news().await?;

    if news.is_empty() {
        println!("No recent news");
    } else {
        println!("{news:#?}");
    }
    //for news_article in news {

    //}

    Ok(())
}
