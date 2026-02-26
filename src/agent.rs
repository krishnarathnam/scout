use crate::{config::Config, tools};
use anyhow::Result;

pub async fn get_ticker(input: &str) -> Result<String> {
    let config = Config::from_env()?;
    let client = reqwest::Client::new();
    let mut ticker: String = String::new();

    let mut prompt = String::from(
        "You are a financial query parser.

        Your job:
        1) Extract the company name or ticker exactly as written in the user question.
        2) If a ticker symbol is explicitly written by the user, return it.
        3) If the ticker appears to be an NSE stock and does NOT already end with .NS, append .NS to it.
        4) If the ticker already ends with .NS, keep it unchanged.
        5) If only a company name is written, DO NOT guess any ticker.
        6) Never infer or guess ticker symbols.
        7) Split the user request into smaller questions preserving meaning.

        Output ONLY valid JSON:

        {
        \"ticker\": null or \"FINAL_TICKER_VALUE\",
        \"company\": \"EXACT_COMPANY_NAME_OR_NULL\",
        \"questions\": [
            \"sub question 1\"
        ]
        }
        ",
    );

    prompt.push_str(&input);

    let body = serde_json::json!({
        "model": config.model,
        "prompt": prompt,
        "stream": false,
    });

    let response = client.post(&config.ollama_host).json(&body).send().await?;

    if response.status().is_success() {
        let text = response.text().await?;
        //println!("RAW RESPONSE START");
        //println!("{}", text);
        //println!("RAW RESPONSE END");
        let outer: serde_json::Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(e) => {
                println!("{e}");
                return Err(e.into());
            }
        };

        let model_output = outer["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No response field"))?;

        let mut cleaned = model_output.trim();

        if cleaned.starts_with("```") {
            cleaned = cleaned
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        }

        let parsed: serde_json::Value = match serde_json::from_str(cleaned) {
            Ok(value) => value,
            Err(e) => {
                println!("{e}");
                return Err(e.into());
            }
        };

        println!("{parsed:#?}");

        if parsed["ticker"].is_null() {
            if let Some(company) = parsed["company"].as_str() {
                let t = match tools::find_ticker(company) {
                    Some(value) => value,
                    None => {
                        return Err(anyhow::anyhow!(
                            "Cannot find ticker for company: {}",
                            company
                        ));
                    }
                };

                ticker = t;
            } else {
                return Err(anyhow::anyhow!("Company or NSE ticker not provided"));
            }
        } else {
            ticker = parsed["ticker"].to_string();
        }
    } else {
        eprintln!("Failed to get response: {:?}", response.status());
    }

    Ok(ticker)
}
