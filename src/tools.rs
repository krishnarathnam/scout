use crate::display;
use crate::income_statement;
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

pub async fn get_income_statement(symbol: &str, client: &reqwest::Client) -> Result<()> {
    let link = format!("https://finance.yahoo.com/quote/{}/financials", symbol);
    let response = client.get(&link).send().await?;

    if !response.status().is_success() {
        println!("Could not fetch data: HTTP {}", response.status());
        return Err(anyhow::anyhow!("Request failed: {}", response.status()));
    }

    let body = response.text().await?;

    if let Some((headers, rows)) = income_statement::scrape_financials_table(&body) {
        let period_headers: Vec<String> = headers.into_iter().skip(1).collect();
        display::print_scraped_table(
            &format!("Income Statement – {}", symbol),
            &period_headers,
            &rows,
        );
        return Ok(());
    }

    let quote_link = format!("https://finance.yahoo.com/quote/{}", symbol);
    let quote_resp = client.get(&quote_link).send().await?;
    if quote_resp.status().is_success() {
        let quote_body = quote_resp.text().await?;
        if let Some(qs) = income_statement::extract_quote_summary(&quote_body) {
            display::print_company_info(symbol, &qs);
            display::print_financials_chart(&qs);
            display::print_income_statement_quarterly(&qs);
            return Ok(());
        }
    }

    Err(anyhow::anyhow!(
        "Could not parse financial data (Yahoo may have changed their format)"
    ))
}
