use anyhow::Result;
use yfinance_rs::{Interval, Range, Ticker, YfClient};

use crate::config::Config;

pub async fn send_message(input: &str) -> Result<()> {
    let config = Config::from_env()?;

    let client = reqwest::Client::new();

    let mut prompt = String::from(
        "You are a financial query parser.

        Your job:
        1) Extract the company name or ticker mentioned in the user's question.
        2) Convert it to the correct NSE ticker ONLY if you are highly confident .
        3) If you are NOT sure of the ticker, return the company name instead and set \"ticker\": \"NONE\" THIS IS HIGHLY IMPORTANT.

        STRICT RULES:
        - Never guess a ticker.
        - If multiple companies could match, return null.
        - Only output a ticker if confidence is very high.
        - Do NOT invent symbols.
        - Do NOT change the company to a different company.

        Also split the user request into smaller questions preserving meaning.

        Output ONLY valid JSON:

        {
        \"ticker\": \"STRING_OR_NULL\",
        \"company\": \"NAME_OR_NULL\",
        \"questions\": [
            \"sub question 1\",
            \"sub question 2\"
        ]             
        }",
    );

    prompt.push_str(&input);

    let body = serde_json::json!({
        "model": config.model,
        "prompt": prompt,
    });

    let response = client.post(&config.ollama_host).json(&body).send().await?;
    let mut full_response = String::new();

    if response.status().is_success() {
        let text = response.text().await?;

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let v: serde_json::Value = serde_json::from_str(line)?;
            if let Some(token) = v["response"].as_str() {
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                full_response.push_str(token);
            }
        }

        let parsed: serde_json::Value = serde_json::from_str(&full_response)?;

        println!("{:#}", parsed);

        if parsed["ticker"].as_str() == Some("NONE") {
            let url = format!(
                "https://query1.finance.yahoo.com/v1/finance/search?q={}",
                parsed["company"]
            );

            let res = client
                .get(url)
                .header("User-Agent", "Mozilla/5.0")
                .send()
                .await?;
            println!("{res:?}");
        }
        //if let Some(ticker) = parsed["ticker"].as_str() {
        //    get_ticker_info(ticker).await?
        //}
    } else {
        eprintln!("Failed to get response: {:?}", response.status());
    }

    Ok(())
}

async fn get_ticker_info(symbol: &str) -> Result<()> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, symbol);

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
    println!("{news:?}");

    Ok(())
}
