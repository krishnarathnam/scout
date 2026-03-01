use crate::yahoo::YahooProvider;
use anyhow::Result;
use csv::Reader;
use std::fs::File;
use std::time::{Duration, Instant};
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

pub async fn get_balance_sheet(symbol: &str, user: &mut YahooProvider) -> Result<()> {
    let min_interval = Duration::from_secs(60);

    if let Some(last) = user.last_requested_time {
        let elapsed = last.elapsed();
        if elapsed < min_interval {
            let remaining = min_interval - elapsed;
            println!(
                "Rate limit active. Try again in {} seconds.",
                remaining.as_secs()
            );
            return Ok(());
        }
    }

    user.last_requested_time = Some(Instant::now());

    let crumb = user
        .crumb
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Missing crumb"))?;

    let link = format!(
        "https://query1.finance.yahoo.com/v10/finance/quoteSummary/NVDA?modules=incomeStatementHistory&crumb={}",
        crumb
    );

    //let link = format!("https://finance.yahoo.com/quote/{symbol}/financials/");

    println!("{link}");

    let response = user.client.get(&link).send().await?;
    println!("Status: {}", response.status());

    let body = response.text().await?;
    println!("{body:#}");

    Ok(())
}
