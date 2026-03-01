use crate::config::Config;
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    event::{read, Event, KeyCode, KeyEventKind},
    execute,
    style::{Color, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};

const PROVIDERS: &[(&str, bool)] = &[
    ("OpenAI", false),
    ("Anthropic", false),
    ("Google", false),
    ("xAI", false),
    ("Moonshot", false),
    ("DeepSeek", false),
    ("OpenRouter", false),
    ("Ollama", true),
];

fn draw_provider_menu(selection: usize) -> Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

    let mut row = 0u16;

    execute!(stdout, SetForegroundColor(Color::Blue))?;
    execute!(stdout, MoveTo(0, row))?;
    writeln!(stdout, "Select provider")?;
    row += 1;
    execute!(stdout, MoveTo(0, row))?;
    writeln!(stdout, "Switch between LLM providers. Applies to this session and future sessions.")?;
    row += 2;

    execute!(stdout, ResetColor)?;
    for (i, (name, implemented)) in PROVIDERS.iter().enumerate() {
        let marker = if i == selection { "> " } else { "  " };
        let suffix = if *implemented {
            ""
        } else {
            " (n/a)"
        };
        let line = format!("{}{}. {}{}", marker, i + 1, name, suffix);
        execute!(stdout, MoveTo(0, row))?;
        if i == selection {
            execute!(stdout, SetForegroundColor(Color::Blue))?;
        }
        writeln!(stdout, "{}", line)?;
        execute!(stdout, ResetColor)?;
        row += 1;
    }

    row += 1;
    execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
    execute!(stdout, MoveTo(0, row))?;
    writeln!(stdout, "Enter to confirm, esc to exit")?;
    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    Ok(())
}

fn draw_model_menu(models: &[String], selection: usize) -> Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

    let mut row = 0u16;

    execute!(stdout, SetForegroundColor(Color::Blue))?;
    execute!(stdout, MoveTo(0, row))?;
    writeln!(stdout, "Select Ollama model")?;
    row += 1;
    execute!(stdout, MoveTo(0, row))?;
    writeln!(stdout, "Choose a model installed on this machine.")?;
    row += 2;

    execute!(stdout, ResetColor)?;
    for (i, name) in models.iter().enumerate() {
        let marker = if i == selection { "> " } else { "  " };
        let line = format!("{}{}. {}", marker, i + 1, name);
        execute!(stdout, MoveTo(0, row))?;
        if i == selection {
            execute!(stdout, SetForegroundColor(Color::Blue))?;
        }
        writeln!(stdout, "{}", line)?;
        execute!(stdout, ResetColor)?;
        row += 1;
    }

    row += 1;
    execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
    execute!(stdout, MoveTo(0, row))?;
    writeln!(stdout, "Enter to confirm, esc to go back")?;
    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    Ok(())
}

async fn fetch_ollama_models(base_url: &str) -> Result<Vec<String>> {
    let url = format!("{}/api/tags", base_url);
    let client = reqwest::Client::new();
    let res = client.get(&url).send().await?;
    let json: serde_json::Value = res.json().await?;
    let models = json["models"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid Ollama tags response"))?
        .iter()
        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();
    Ok(models)
}

pub async fn run_model_selection() -> Result<()> {
    let mut selection = PROVIDERS.len().saturating_sub(1);
    loop {
        draw_provider_menu(selection)?;
        match read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up => {
                    selection = if selection == 0 {
                        PROVIDERS.len().saturating_sub(1)
                    } else {
                        selection - 1
                    };
                }
                KeyCode::Down => {
                    selection = if selection >= PROVIDERS.len().saturating_sub(1) {
                        0
                    } else {
                        selection + 1
                    };
                }
                KeyCode::Enter => {
                    let (_, implemented) = PROVIDERS[selection];
                    if !implemented {
                        continue;
                    }
                    if selection == 7 {
                        break;
                    }
                }
                KeyCode::Esc => return Ok(()),
                _ => {}
            },
            _ => {}
        }
    }

    let config = Config::from_env()?;
    let base_url = config.ollama_base_url().to_string();

    let models = match fetch_ollama_models(&base_url).await {
        Ok(m) if m.is_empty() => {
            let mut stdout = io::stdout();
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
            writeln!(stdout, "No Ollama models found. Install models with: ollama pull <name>")?;
            writeln!(stdout, "\nPress any key to go back.")?;
            stdout.flush()?;
            let _ = read()?;
            return Ok(());
        }
        Ok(m) => m,
        Err(e) => {
            let mut stdout = io::stdout();
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
            writeln!(stdout, "Could not reach Ollama at {}. Error: {}", base_url, e)?;
            writeln!(stdout, "\nPress any key to go back.")?;
            stdout.flush()?;
            let _ = read()?;
            return Ok(());
        }
    };

    let mut model_selection = 0usize;
    loop {
        draw_model_menu(&models, model_selection)?;
        match read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up => {
                    model_selection = if model_selection == 0 {
                        models.len().saturating_sub(1)
                    } else {
                        model_selection - 1
                    };
                }
                KeyCode::Down => {
                    model_selection = if model_selection >= models.len().saturating_sub(1) {
                        0
                    } else {
                        model_selection + 1
                    };
                }
                KeyCode::Enter => {
                    let config = Config::from_env()?;
                    let chosen = models[model_selection].clone();
                    config.save_model(&chosen)?;
                    let mut stdout = io::stdout();
                    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
                    writeln!(stdout, "Model set to: {}", chosen)?;
                    writeln!(stdout, "\nPress any key to continue.")?;
                    stdout.flush()?;
                    let _ = read()?;
                    return Ok(());
                }
                KeyCode::Esc => return Ok(()),
                _ => {}
            },
            _ => {}
        }
    }
}
