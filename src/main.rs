mod config;
use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    event::{read, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
};
use std::io::{self, Write};

use crate::config::Config;

fn redraw(input: &str, prev_lines: &mut u16) {
    let mut stdout = io::stdout();

    let prompt = format!("> {}", input);
    let (cols, _) = size().unwrap_or((80, 0));
    let cols = cols.max(1);
    let content_len = prompt.chars().count() as u16;
    let lines = ((content_len.saturating_sub(1)) / cols) + 1;

    if *prev_lines > 1 {
        for _ in 0..(*prev_lines - 1) {
            execute!(stdout, MoveUp(1)).unwrap();
        }
    }

    execute!(stdout, MoveToColumn(0), Clear(ClearType::FromCursorDown)).unwrap();

    print!("{}", prompt);
    stdout.flush().unwrap();

    *prev_lines = lines;
}

async fn send_message(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    println!("Using model:{} on {}", config.model, config.ollama_host);

    let client = reqwest::Client::new();

    let mut prompt = String::from(
        "You are a query parser for a financial CLI tool.

        Your job:
        1) Extract the NSE ticker symbol mentioned in the user's question.
        2) If a company name is used, infer the correct stock ticker.
        3) Split the user's question into smaller logical sub-questions that preserve the original meaning.

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
        "steam": false
    });

    let response = client.post(&config.ollama_host).json(&body).send().await?;

    if response.status().is_success() {
        let text = response.text().await?;

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let v: serde_json::Value = serde_json::from_str(line)?;
            if let Some(token) = v["response"].as_str() {
                print!("{}", token);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
        }

        println!();
    } else {
        eprintln!("Failed to get response: {:?}", response.status());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;

    let mut input = String::new();
    let mut prev_lines: u16 = 1;
    redraw(&input, &mut prev_lines);

    loop {
        match read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char(c) => {
                    input.push(c);
                    redraw(&input, &mut prev_lines);
                }
                KeyCode::Backspace => {
                    input.pop();
                    redraw(&input, &mut prev_lines);
                }

                KeyCode::Enter => {
                    println!();

                    disable_raw_mode()?;
                    send_message(&input).await?;
                    enable_raw_mode()?;
                    input.clear();
                    prev_lines = 1;

                    let mut stdout = io::stdout();
                    print!("> ");
                    stdout.flush().unwrap();
                }

                KeyCode::Esc => break,

                _ => {}
            },
            _ => {}
        }
    }

    disable_raw_mode()?;
    Ok(())
}
