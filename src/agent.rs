use crate::{config::Config, tools};
use anyhow::{Ok, Result};

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
        let outer: serde_json::Value = match serde_json::from_str(&text) {
            std::result::Result::Ok(value) => value,
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
            std::result::Result::Ok(value) => value,
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

                ticker = t.to_string();
            } else {
                return Err(anyhow::anyhow!("Company or NSE ticker not provided"));
            }
        } else {
            ticker = parsed["ticker"].as_str().unwrap().to_string();
        }
    } else {
        eprintln!("Failed to get response: {:?}", response.status());
    }
    Ok(ticker)
}

pub async fn get_financial_review(finance_statement: &String) -> Result<()> {
    println!("- Analyzing data");
    let config = Config::from_env()?;
    let client = reqwest::Client::new();
    let mut prompt: String = String::from("You are a financial statement analyzer.

    You will be given structured financial data for a company’s:

    1) Balance sheet  
    2) Income statement  
    3) Cash flow

    Your task is to generate a **four-paragraph plain English analysis**, one for each of the following:

    Paragraph 1: Balance sheet insights  
    Paragraph 2: Income statement insights  
    Paragraph 3: Cash flow insights  
    Paragraph 4: Based on the given insights, provide a suggestion whether investing in this company appears favorable or not.

    Rules:

    • Use only the numbers present in the input — do NOT add any external knowledge or guess anything.  
    • Do NOT hallucinate metrics that are not in the data.  
    • Do NOT explain how you generated the text — output only the final analysis text.  
    • Each paragraph should reference the key trends or relationships seen in the provided numbers.  
    • If a section has missing fields, mention that fact explicitly without guessing the missing numbers.  
    • The final paragraph must be based strictly on the insights from the three earlier paragraphs and the given data — do NOT introduce new information.

    Here is the input data:");

    prompt.push_str(&finance_statement);

    let body = serde_json::json!({
        "model": config.model,
        "prompt": prompt,
        "stream": false,
    });

    let response = client.post(&config.ollama_host).json(&body).send().await?;

    if response.status().is_success() {
        let text = response.text().await?;
        let outer: serde_json::Value = match serde_json::from_str(&text) {
            std::result::Result::Ok(value) => value,
            Err(e) => {
                println!("{e}");
                return Err(e.into());
            }
        };

        let model_output = outer["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No response field"))?;

        let cleaned = model_output.trim().to_string();

        println!("{cleaned}");
    } else {
        eprintln!("Failed to get response: {:?}", response.status());
    }

    Ok(())
}

pub async fn get_news_review(news: &String) -> Result<()> {
    let config = Config::from_env()?;
    let client = reqwest::Client::new();
    let mut prompt: String = String::from("You are a financial news analyst.

You will be given multiple news headlines or article summaries related to a company.

Your task is to analyze ALL the news together and generate the following output:

Section 1: Key events summary (bullet points)  
Section 2: Sentiment analysis (bullet points)  
Section 3: Overall interpretation paragraph

Format:

Key Developments:
- point
- point
- point

News Sentiment:
- point
- point
- point

Overall Impact:
Write one concise paragraph explaining the overall meaning of the news and the potential impact on investor perception.

Rules:
• Analyze all news items collectively.  
• Identify repeated themes, risks, announcements, or events.  
• Use only information present in the provided news.  
• Do NOT hallucinate facts or introduce outside knowledge.  
• If news items conflict, explicitly mention the contradiction.  
• If the information is insufficient, state that clearly.

Here is the news data:");

    prompt.push_str(&news);
    let body = serde_json::json!({
        "model": config.model,
        "prompt": prompt,
        "stream": false,
    });

    let response = client.post(&config.ollama_host).json(&body).send().await?;

    if response.status().is_success() {
        let text = response.text().await?;
        let outer: serde_json::Value = match serde_json::from_str(&text) {
            std::result::Result::Ok(value) => value,
            Err(e) => {
                println!("{e}");
                return Err(e.into());
            }
        };

        let model_output = outer["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No response field"))?;

        let cleaned = model_output.trim().to_string();

        println!("{cleaned}");
    } else {
        eprintln!("Failed to get response: {:?}", response.status());
    }
    Ok(())
}
