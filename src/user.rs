use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
pub fn user_client() -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();

    // Mimic real browser headers
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        ),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://finance.yahoo.com/"),
    );

    let client = match reqwest::Client::builder()
        .default_headers(headers.clone())
        .build()
    {
        Ok(cli) => cli,
        Err(e) => return Err(e.into()),
    };

    Ok(client)
}
