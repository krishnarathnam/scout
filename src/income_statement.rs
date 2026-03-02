/// Scrape the income statement table from the qsp-financials section.
pub fn scrape_financials_table(
    html: &str,
) -> Option<(Vec<String>, Vec<(String, Vec<String>)>)> {
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

/// Fallback: scan for "quoteSummary": and extract the JSON value (handles nested { } [ ]).
fn extract_quote_summary_fallback(html: &str) -> Option<serde_json::Value> {
    let start = html.find("\"quoteSummary\"")?;
    let rest = &html[start + 14..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    if !after_colon.starts_with('{') {
        return None;
    }
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
pub fn extract_quote_summary(html: &str) -> Option<serde_json::Value> {
    let doc = scraper::Html::parse_document(html);

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
