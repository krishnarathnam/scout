mod agent;
mod config;
use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    event::{Event, KeyCode, KeyEventKind, read},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode, size},
};
use std::io::{self, Write};

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
                    println!("\n");

                    disable_raw_mode()?;
                    agent::send_message(&input).await?;
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
