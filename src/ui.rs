use anyhow::Result;
use crossterm::{
    cursor::{MoveTo, MoveToColumn, MoveUp},
    execute,
    style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{size, Clear, ClearType},
};
use std::io::{self, Write};

fn art_lines() -> Vec<&'static str> {
    vec![
        " ██████╗ ██████╗   ██████╗ ██╗   ██╗████████╗",
        "██╔════╝ ██╔════╝ ██╔═══██╗██║   ██║╚══██╔══╝",
        "╚█████╗  ██║      ██║   ██║██║   ██║   ██║   ",
        " ╚═══██╗ ██║      ██║   ██║██║   ██║   ██║   ",
        "██████╔╝ ╚██████╗ ╚██████╔╝╚██████╔╝   ██║   ",
        "╚═════╝   ╚═════╝  ╚═════╝  ╚═════╝    ╚═╝   ",
    ]
}

pub fn print_banner() -> Result<()> {
    let mut stdout = io::stdout();
    let (cols, _) = size().unwrap_or((80, 0));

    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

    // Shadow layer (slightly offset)
    let art = art_lines();
    for (i, line) in art.iter().enumerate() {
        execute!(
            stdout,
            MoveTo(4 + 1, i as u16 + 2 + 1),
            SetForegroundColor(Color::DarkGrey)
        )?;
        writeln!(stdout, "{line}")?;
    }

    // Main bright layer
    execute!(
        stdout,
        SetForegroundColor(Color::Blue),
        SetAttribute(Attribute::Bold)
    )?;
    for (i, line) in art.iter().enumerate() {
        execute!(stdout, MoveTo(4, i as u16 + 2))?;
        writeln!(stdout, "{line}")?;
    }
    execute!(stdout, SetAttribute(Attribute::Reset), ResetColor)?;

    // Subtitle & hint text
    let subtitle = "Stock Company Oracle Utility Terminal";
    let hint_1 = "Type a company or ticker and press Enter.";
    let hint_2 = "Type /model to change LLM.";
    let hint_3 = "Press Esc to exit.";

    let art_height = art.len() as u16;
    let text_y = art_height + 4;

    let safe_cols = cols.max(60);
    let max_width = safe_cols.saturating_sub(8) as usize;
    let truncate = |s: &str| {
        if s.chars().count() > max_width {
            let mut out = String::new();
            for (i, ch) in s.chars().enumerate() {
                if i >= max_width.saturating_sub(3) {
                    out.push_str("...");
                    break;
                }
                out.push(ch);
            }
            out
        } else {
            s.to_string()
        }
    };

    execute!(stdout, SetForegroundColor(Color::Grey))?;
    execute!(stdout, MoveTo(4, text_y))?;
    writeln!(stdout, "{}", truncate(subtitle))?;
    execute!(stdout, MoveTo(4, text_y + 2))?;
    writeln!(stdout, "{}", truncate(hint_1))?;
    execute!(stdout, MoveTo(4, text_y + 3))?;
    writeln!(stdout, "{}", truncate(hint_2))?;
    execute!(stdout, MoveTo(4, text_y + 4))?;
    writeln!(stdout, "{}", truncate(hint_3))?;
    execute!(stdout, ResetColor)?;

    writeln!(stdout)?;
    stdout.flush()?;

    Ok(())
}

pub fn redraw(input: &str, prev_lines: &mut u16) {
    let mut stdout = io::stdout();

    let prompt = format!("SCOUT > {}", input);
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

    execute!(stdout, SetForegroundColor(Color::White)).unwrap();
    print!("{}", prompt);
    execute!(stdout, ResetColor).unwrap();
    stdout.flush().unwrap();

    *prev_lines = lines;
}

