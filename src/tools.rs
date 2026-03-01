use crate::yahoo::YahooProvider;
use anyhow::Result;
use csv::Reader;
use std::fs::File;
use std::time::Instant;
use strsim::jaro_winkler;

pub fn find_ticker(company: &str) -> Option<String> {
    let file = File::open("data/nse.csv").ok()?;
    let mut rdr = Reader::from_reader(file);

    let mut best = None;
    let mut best_score = 0.0;

    for row in rdr.records() {
        let r = row.ok()?;
        let symbol = &r[0];
        let name = &r[1];

        let score = jaro_winkler(&company.to_lowercase(), &name.to_lowercase());

        if score > best_score {
            best_score = score;
            best = Some(symbol.to_string());
        }
    }

    if best_score > 0.80 {
        best.map(|s| format!("{}.NS", s))
    } else {
        None
    }
}

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

/// Scrape the income statement table from the qsp-financials section.
fn scrape_financials_table(html: &str) -> Option<(Vec<String>, Vec<(String, Vec<String>)>)> {
    let doc = scraper::Html::parse_document(html);
    let section_sel = scraper::Selector::parse(r#"[data-testid="qsp-financials"]"#).ok()?;
    let section = doc.select(&section_sel).next()?;

    let header_sel = scraper::Selector::parse(".tableHeader .row .column").ok()?;
    let row_sel = scraper::Selector::parse(".tableBody .row").ok()?;
    let column_sel = scraper::Selector::parse(".column").ok()?;

    let headers: Vec<String> = section
        .select(&header_sel)
        .map(|el| el.text().collect::<String>().trim().to_string())
        .collect();

    let mut rows = Vec::new();
    for row_el in section.select(&row_sel) {
        let cols: Vec<String> = row_el
            .select(&column_sel)
            .map(|c| c.text().collect::<String>().trim().to_string())
            .collect();

        if cols.is_empty() {
            continue;
        }
        // First column has the label (rowTitle), rest are values
        let label = cols.first()?.clone();
        let values: Vec<String> = cols.into_iter().skip(1).collect();
        if !label.is_empty() {
            rows.push((label, values));
        }
    }

    if headers.is_empty() || rows.is_empty() {
        return None;
    }
    Some((headers, rows))
}

fn print_scraped_table(title: &str, headers: &[String], rows: &[(String, Vec<String>)]) {
    if rows.is_empty() || headers.is_empty() {
        return;
    }
    let col_width = 14usize;
    let header_width = 40usize;
    let sep = "-".repeat(header_width + 1 + headers.len() * (col_width + 1));
    println!("\n{}\n{}", title, sep);
    print!("{:>width$} |", headers.first().cloned().unwrap_or_else(|| "Period".into()), width = header_width);
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

/// Fallback: scan for "quoteSummary": and extract the JSON value (handles nested { } [ ]).
fn extract_quote_summary_fallback(html: &str) -> Option<serde_json::Value> {
    let start = html.find("\"quoteSummary\"")?;
    let rest = &html[start + 14..]; // skip "quoteSummary"
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    if !after_colon.starts_with('{') {
        return None;
    }
    // Match braces and brackets to find the full JSON value
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escape = false;
    let mut quote_char = '\0';
    let chars: Vec<char> = after_colon.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
                continue;
            }
            if c == quote_char {
                in_string = false;
            }
            continue;
        }
        if c == '"' || c == '\'' {
            in_string = true;
            quote_char = c;
            continue;
        }
        if c == '{' || c == '[' {
            stack.push(if c == '{' { '}' } else { ']' });
            continue;
        }
        if c == '}' || c == ']' {
            if stack.pop() != Some(c) {
                return None;
            }
            if stack.is_empty() {
                let json_str: String = chars[..=i].iter().collect();
                return serde_json::from_str(&json_str).ok();
            }
        }
    }
    None
}

/// Extract quoteSummary JSON from embedded script tags in Yahoo Finance HTML.
fn extract_quote_summary(html: &str) -> Option<serde_json::Value> {
    let doc = scraper::Html::parse_document(html);

    // Try script tags (both type=application/json and data-sveltekit-fetched)
    for selector in &[
        "script[type=\"application/json\"]",
        "script[data-sveltekit-fetched]",
        "script",
    ] {
        let Ok(sel) = scraper::Selector::parse(selector) else { continue };
        for script in doc.select(&sel) {
            let text = script.text().collect::<String>();
            if !text.contains("quoteSummary") {
                continue;
            }
            // Format: {"status":200,"body":"{\"quoteSummary\":{...}}"}
            if let Ok(outer) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(body_str) = outer.get("body").and_then(|b| b.as_str()) {
                    if let Ok(inner) = serde_json::from_str::<serde_json::Value>(body_str) {
                        if let Some(qs) = inner.get("quoteSummary") {
                            return Some(qs.clone());
                        }
                    }
                }
            }
        }
    }

    extract_quote_summary_fallback(html)
}

fn print_company_info(symbol: &str, data: &serde_json::Value) {
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

fn get_fmt(v: &serde_json::Value) -> String {
    v.get("fmt")
        .and_then(|f| f.as_str())
        .unwrap_or("-")
        .to_string()
}

fn print_financials_chart(data: &serde_json::Value) {
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

fn print_income_statement_quarterly(data: &serde_json::Value) {
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
        let end_date = stmt.get("endDate").and_then(|d| d.get("fmt")).and_then(|f| f.as_str()).unwrap_or("-");
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

pub async fn get_balance_sheet(symbol: &str, user: &mut YahooProvider) -> Result<()> {
    user.last_requested_time = Some(Instant::now());

    let link = format!("https://finance.yahoo.com/quote/{}/financials", symbol);
    let response = user.client.get(&link).send().await?;

    if !response.status().is_success() {
        println!("Could not fetch data: HTTP {}", response.status());
        return Err(anyhow::anyhow!("Request failed: {}", response.status()));
    }

    let body = response.text().await?;

    if let Some((headers, rows)) = scrape_financials_table(&body) {
        let period_headers: Vec<String> = headers.into_iter().skip(1).collect();
        print_scraped_table(
            &format!("Income Statement – {}", symbol),
            &period_headers,
            &rows,
        );
        return Ok(());
    }

    // Fallback: try quote summary from main quote page (for NSE symbols etc.)
    let quote_link = format!("https://finance.yahoo.com/quote/{}", symbol);
    let quote_resp = user.client.get(&quote_link).send().await?;
    if quote_resp.status().is_success() {
        let quote_body = quote_resp.text().await?;
        if let Some(qs) = extract_quote_summary(&quote_body) {
            print_company_info(symbol, &qs);
            print_financials_chart(&qs);
            print_income_statement_quarterly(&qs);
            return Ok(());
        }
    }

    Err(anyhow::anyhow!(
        "Could not parse financial data (Yahoo may have changed their format)"
    ))
}
