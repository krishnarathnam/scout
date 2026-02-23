mod config;
use crossterm::{
    cursor::MoveToColumn,
    event::{Event, KeyCode, KeyEventKind, read},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};

use crate::config::Config;

fn redraw(input: &str) {
    let mut stdout = io::stdout();
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine)).unwrap();

    print!("> {}", input);
    stdout.flush().unwrap();
}

async fn send_message(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    println!("Using model:{} on {}", config.model, config.ollama_host);

    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": config.model,
        "prompt": input,
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
    redraw(&input);

    loop {
        match read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char(c) => {
                    input.push(c);
                    redraw(&input);
                }

                KeyCode::Backspace => {
                    input.pop();
                    redraw(&input);
                }

                KeyCode::Enter => {
                    println!();

                    disable_raw_mode()?;
                    send_message(&input).await?;
                    enable_raw_mode()?;
                    input.clear();
                    redraw(&input);
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
