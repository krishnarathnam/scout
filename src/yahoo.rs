use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, REFERER};
use std::time::Instant;

pub struct YahooProvider {
    pub client: reqwest::Client,
    pub crumb: Option<String>,
    pub last_requested_time: Option<Instant>,
}

impl YahooProvider {
    pub async fn new() -> anyhow::Result<Self> {
        let client = match reqwest::Client::builder()
            .cookie_store(true)
            .user_agent("Mozilla/5.0")
            .build()
        {
            Ok(value) => value,
            Err(e) => {
                println!("{e}");
                return Err(e.into());
            }
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://finance.yahoo.com/"),
        );

        client
            .get("https://finance.yahoo.com")
            .headers(headers.clone())
            .send()
            .await?;

        let response = match client
            .get("https://query1.finance.yahoo.com/v1/test/getcrumb")
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                println!("Error during send(): {e}");
                return Err(e.into());
            }
        };

        println!("Status: {}", response.status());

        let crumb_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                println!("Error during text(): {e}");
                return Err(e.into());
            }
        };

        let crumb = crumb_text.trim().to_string();

        println!("CRUMB: {}", crumb);

        Ok(Self {
            client,
            crumb: Some(crumb),
            last_requested_time: None,
        })
    }
}
