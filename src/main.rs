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
                        input.clear();
                        prev_lines = 1;
                        ui::print_banner()?;
                        ui::redraw(&input, &mut prev_lines);
                        continue;
                    }
                    disable_raw_mode()?;
                    println!();

                    let ticker = match agent::get_ticker(&input).await {
                        Ok(value) => value,
                        Err(e) => {
                            println!("{e}");
                            enable_raw_mode()?;
                            input.clear();
                            prev_lines = 1;
                            ui::redraw(&input, &mut prev_lines);
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

                    let _news = match news_res {
                        Ok(val) => val,
                        Err(e) => {
                            println!("{e}");
                            enable_raw_mode()?;
                            input.clear();
                            prev_lines = 1;
                            ui::redraw(&input, &mut prev_lines);
                            continue;
                        }
                    };


                    match inc_res {
                        Ok(val) => output.push_str(val.as_str()),
                        Err(e) => {
                            println!("{e}");
                            enable_raw_mode()?;
                            input.clear();
                            prev_lines = 1;
                            ui::redraw(&input, &mut prev_lines);
                            continue;
                        }
                    };

                    match bal_res {
                        Ok(val) => output.push_str(val.as_str()),
                        Err(e) => {
                            println!("{e}");
                            enable_raw_mode()?;
                            input.clear();
                            prev_lines = 1;
                            ui::redraw(&input, &mut prev_lines);
                            continue;
                        }
                    };

                    match cash_res {
                        Ok(val) => output.push_str(val.as_str()),
                        Err(e) => {
                            println!("{e}");
                            enable_raw_mode()?;
                            input.clear();
                            prev_lines = 1;
                            ui::redraw(&input, &mut prev_lines);
                            continue;
                        }
                    };

                match agent::get_review(&output).await {
                    Ok(val) => val,
                    Err(e) => {
                        println!("{e}");
                        enable_raw_mode()?;
                        input.clear();
                        prev_lines = 1;
                        ui::redraw(&input, &mut prev_lines);
                        continue;
                        }
                    }

                    enable_raw_mode()?;
                    input.clear();
                    prev_lines = 1;
                    ui::redraw(&input, &mut prev_lines);
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
