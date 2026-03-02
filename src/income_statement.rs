/// Scrape the income statement table from the qsp-financials section.
pub fn scrape_financials_table(html: &str) -> Option<(Vec<String>, Vec<(String, Vec<String>)>)> {
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
