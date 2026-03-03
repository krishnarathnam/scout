use std::fmt::Write;

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

pub fn print_scraped_table(
    title: &str,
    headers: &[String],
    rows: &[(String, Vec<String>)],
) -> String {
    if rows.is_empty() || headers.is_empty() {
        return String::new();
    }
    let col_width = 14usize;
    let header_width = 40usize;
    let sep = "-".repeat(header_width + 1 + headers.len() * (col_width + 1));

    let mut out = String::new();
    writeln!(&mut out, "\n{}\n{}", title, sep).ok();
    write!(&mut out, "{:>width$} |", "Metric", width = header_width).ok();
    for h in headers.iter() {
        let h = if h.len() > col_width {
            format!("{}..", &h[..col_width.saturating_sub(2)])
        } else {
            h.clone()
        };
        write!(&mut out, " {:>width$} |", h, width = col_width).ok();
    }
    writeln!(&mut out).ok();
    writeln!(&mut out, "{}", sep).ok();
    for (label, cells) in rows {
        let label_trim = if label.len() > header_width {
            format!("{}..", &label[..header_width.saturating_sub(2)])
        } else {
            label.clone()
        };
        write!(&mut out, "{:>width$} |", label_trim, width = header_width).ok();
        for c in cells {
            write!(&mut out, " {:>width$} |", format_cell(c), width = col_width).ok();
        }
        writeln!(&mut out).ok();
    }
    writeln!(&mut out, "{}", sep).ok();

    out
}
