use anyhow::Result;
use yfinance_rs::{Interval, Range, Ticker, YfClient};

use crate::config::Config;

pub async fn send_message(input: &str) -> Result<()> {
    let config = Config::from_env()?;

    let client = reqwest::Client::new();

    let mut prompt = String::from(
        "You are a query parser for a financial CLI tool.

        Your job:
        1) Extract the ticker symbol mentioned in the user's question.
        2) If a company name is used, infer the correct stock ticker check the web to get the ticker information.
        3) Split the user's question into smaller logical sub-questions that preserve the original meaning.
        4) If the ticker symbol is in NSE add .NS next to

        IMPORTANT RULES:
        - Do NOT invent data retrieval tasks.
        - Do NOT mention APIs, scraping, reddit, news, financial statements, or analysis methods.
        - Only rewrite the user's intent into simpler questions.
        - Keep the meaning exactly the same.
        - Output ONLY valid JSON.
        - No explanations, no markdown.

        Return format:

        {
        \"ticker\": \"STRING_OR_NULL\",
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
        "stream": false
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
                //print!("{}", token);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                full_response.push_str(token);
            }
        }

        let parsed: serde_json::Value = serde_json::from_str(&full_response)?;

        println!("{:?}", parsed);

        if let Some(ticker) = parsed["ticker"].as_str() {
            get_ticker_info(ticker).await?
        }
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
