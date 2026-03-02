fn format_num(n: f64) -> String {
    let abs = n.abs();
    let sign = if n < 0.0 { "-" } else { "" };
    let (val, suffix) = if abs >= 1e12 {
        (abs / 1e12, "T")
    } else if abs >= 1e9 {
        (abs / 1e9, "B")
    } else if abs >= 1e6 {
        (abs / 1e6, "M")
    } else if abs >= 1e3 {
        (abs / 1e3, "K")
    } else {
        (abs, "")
    };
    if suffix.is_empty() {
        format!("{sign}{val:.2}")
    } else {
        format!("{sign}{val:.2}{suffix}")
    }
}

fn format_cell(s: &str) -> String {
    let s = s.trim().replace(',', "");
    if s == "--" || s.is_empty() {
        return "-".to_string();
    }
    if let Ok(n) = s.parse::<f64>() {
        format_num(n)
    } else {
        s.to_string()
    }
}

fn _get_raw(v: &serde_json::Value) -> Option<f64> {
    v.get("raw")?.as_f64()
}

fn _get_fmt(v: &serde_json::Value) -> String {
    v.get("fmt")
        .and_then(|f| f.as_str())
        .unwrap_or("-")
        .to_string()
}

pub fn print_scraped_table(title: &str, headers: &[String], rows: &[(String, Vec<String>)]) {
    if rows.is_empty() || headers.is_empty() {
        return;
    }
    let col_width = 14usize;
    let header_width = 40usize;
    let sep = "-".repeat(header_width + 1 + headers.len() * (col_width + 1));
    println!("\n{}\n{}", title, sep);
    print!("{:>width$} |", "Metric", width = header_width);
    for h in headers.iter() {
        let h = if h.len() > col_width {
            format!("{}..", &h[..col_width.saturating_sub(2)])
        } else {
            h.clone()
        };
        print!(" {:>width$} |", h, width = col_width);
    }
    println!();
    println!("{}", sep);
    for (label, cells) in rows {
        let label_trim = if label.len() > header_width {
            format!("{}..", &label[..header_width.saturating_sub(2)])
        } else {
            label.clone()
        };
        print!("{:>width$} |", label_trim, width = header_width);
        for c in cells {
            print!(" {:>width$} |", format_cell(c), width = col_width);
        }
        println!();
    }
    println!("{}\n", sep);
}
