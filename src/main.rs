mod agent;
mod config;
mod display;
mod income_statement;
mod model_select;
mod tools;
mod ui;
mod user;

use anyhow::Result;
use crossterm::{
    event::{Event, KeyCode, KeyEventKind, read},
    terminal::{disable_raw_mode, enable_raw_mode},
};

fn reset_prompt(input: &mut String, prev_lines: &mut u16) {
    input.clear();
    *prev_lines = 1;
    ui::redraw(input, prev_lines);
}

fn reset_with_banner(input: &mut String, prev_lines: &mut u16) {
    input.clear();
    *prev_lines = 1;
    if let Err(e) = ui::print_banner() {
        eprintln!("{e}");
    }
    ui::redraw(input, prev_lines);
}

fn handle_error<E: std::fmt::Display>(e: E, input: &mut String, prev_lines: &mut u16) {
    println!("{e}");
    if let Err(err) = enable_raw_mode() {
        eprintln!("{err}");
    }
    reset_prompt(input, prev_lines);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = user::user_client()?;
    let news_client = user::user_client()?;

    enable_raw_mode()?;

    ui::print_banner()?;

    let mut input = String::new();
    let mut prev_lines: u16 = 1;
    ui::redraw(&input, &mut prev_lines);

    loop {
        match read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char(c) => {
                    input.push(c);
                    ui::redraw(&input, &mut prev_lines);
                }
                KeyCode::Backspace => {
                    input.pop();
                    ui::redraw(&input, &mut prev_lines);
                }

                KeyCode::Enter => {
                    if input.trim() == "/model" {
                        model_select::run_model_selection().await?;
                        reset_with_banner(&mut input, &mut prev_lines);
                        continue;
                    }
                    disable_raw_mode()?;
                    println!();

                    let ticker = match agent::get_ticker(&input).await {
                        Ok(value) => value,
                        Err(e) => {
                            handle_error(e, &mut input, &mut prev_lines);
                            continue;
                        }
                    };

                    println!("Resolved ticker: {}", ticker);
                    let mut output = String::new();
                    let (inc_res, bal_res, cash_res, news_res) = tokio::join!(
                        tools::get_financials(&ticker, &client, "income_statement"),
                        tools::get_financials(&ticker, &client, "balance_sheet"),
                        tools::get_financials(&ticker, &client, "cash_flow"),
                        tools::get_news(&news_client, &ticker)
                    );

                    let news = match news_res {
                        Ok(val) => val,
                        Err(e) => {
                            handle_error(e, &mut input, &mut prev_lines);
                            continue;
                        }
                    };

                    match inc_res {
                        Ok(val) => output.push_str(val.as_str()),
                        Err(e) => {
                            handle_error(e, &mut input, &mut prev_lines);
                            continue;
                        }
                    };

                    match bal_res {
                        Ok(val) => output.push_str(val.as_str()),
                        Err(e) => {
                            handle_error(e, &mut input, &mut prev_lines);
                            continue;
                        }
                    };

                    match cash_res {
                        Ok(val) => output.push_str(val.as_str()),
                        Err(e) => {
                            handle_error(e, &mut input, &mut prev_lines);
                            continue;
                        }
                    };

                    if let Err(e) = agent::get_financial_review(&output).await {
                        handle_error(e, &mut input, &mut prev_lines);
                        continue;
                    }
                    println!("\n\n");
                    if let Err(e) = agent::get_news_review(&news).await {
                        handle_error(e, &mut input, &mut prev_lines);
                        continue;
                    }
                    if let Err(e) = enable_raw_mode() {
                        eprintln!("{e}");
                    }
                    reset_prompt(&mut input, &mut prev_lines);
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
