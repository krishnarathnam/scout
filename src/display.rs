/// Display and formatting for financial data.

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

fn get_raw(v: &serde_json::Value) -> Option<f64> {
    v.get("raw")?.as_f64()
}

fn get_fmt(v: &serde_json::Value) -> String {
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
    print!(
        "{:>width$} |",
        headers.first().cloned().unwrap_or_else(|| "Period".into()),
        width = header_width
    );
    for h in headers.iter().skip(1) {
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

pub fn print_company_info(symbol: &str, data: &serde_json::Value) {
    let result = match data.get("result").and_then(|r| r.get(0)) {
        Some(r) => r,
        None => return,
    };

    let price = result.get("price");
    let summary = result.get("summaryDetail");

    let name = price
        .and_then(|p| p.get("longName").or(p.get("shortName")))
        .and_then(|n| n.as_str())
        .unwrap_or(symbol);

    println!("\n=== {} ({}) ===", name, symbol);

    if let Some(p) = price {
        if let Some(v) = p.get("regularMarketPrice") {
            println!("  Price: {}", get_fmt(v));
        }
    }

    if let Some(s) = summary {
        let metrics = [
            ("Market Cap", "marketCap", true),
            ("Volume", "volume", true),
            ("Trailing PE", "trailingPE", false),
            ("Forward PE", "forwardPE", false),
            ("Dividend Yield", "dividendYield", false),
        ];
        for (label, key, format_large) in metrics {
            if let Some(v) = s.get(key) {
                if format_large {
                    if let Some(raw) = get_raw(v) {
                        println!("  {}: {}", label, format_num(raw));
                    }
                } else if let Some(fmt) = v.get("fmt").and_then(|f| f.as_str()) {
                    println!("  {}: {}", label, fmt);
                }
            }
        }
    }
}

pub fn print_financials_chart(data: &serde_json::Value) {
    let result = match data.get("result").and_then(|r| r.get(0)) {
        Some(r) => r,
        None => return,
    };

    let earnings = match result.get("earnings") {
        Some(e) => e,
        None => return,
    };

    let chart = match earnings.get("financialsChart") {
        Some(c) => c,
        None => return,
    };

    println!("\n--- Revenue & Earnings (Annual) ---");
    if let Some(yearly) = chart.get("yearly").and_then(|y| y.as_array()) {
        for yr in yearly {
            let date = yr.get("date").and_then(|d| d.as_i64()).unwrap_or(0);
            let rev = yr.get("revenue").and_then(get_raw).unwrap_or(0.0);
            let earn = yr.get("earnings").and_then(get_raw).unwrap_or(0.0);
            println!(
                "  {}: Revenue {}  |  Earnings {}",
                date,
                format_num(rev),
                format_num(earn)
            );
        }
    }

    println!("\n--- Revenue & Earnings (Quarterly) ---");
    if let Some(quarterly) = chart.get("quarterly").and_then(|q| q.as_array()) {
        for q in quarterly {
            let date = q.get("date").and_then(|d| d.as_str()).unwrap_or("-");
            let fiscal = q.get("fiscalQuarter").and_then(|f| f.as_str()).unwrap_or("");
            let rev = q.get("revenue").and_then(get_raw).unwrap_or(0.0);
            let earn = q.get("earnings").and_then(get_raw).unwrap_or(0.0);
            println!(
                "  {} ({}): Revenue {}  |  Earnings {}",
                date,
                fiscal,
                format_num(rev),
                format_num(earn)
            );
        }
    }
}

pub fn print_income_statement_quarterly(data: &serde_json::Value) {
    let result = match data.get("result").and_then(|r| r.get(0)) {
        Some(r) => r,
        None => return,
    };

    let hist = match result.get("incomeStatementHistoryQuarterly") {
        Some(h) => h,
        None => return,
    };

    let statements = match hist.get("incomeStatementHistory").and_then(|s| s.as_array()) {
        Some(s) => s,
        None => return,
    };

    println!("\n--- Income Statement (Quarterly) ---");
    for stmt in statements {
        let end_date = stmt
            .get("endDate")
            .and_then(|d| d.get("fmt"))
            .and_then(|f| f.as_str())
            .unwrap_or("-");
        let revenue = stmt.get("totalRevenue").and_then(get_raw).unwrap_or(0.0);
        let net_income = stmt.get("netIncome").and_then(get_raw).unwrap_or(0.0);
        println!(
            "  {}: Revenue {}  |  Net Income {}",
            end_date,
            format_num(revenue),
            format_num(net_income)
        );
    }
}
