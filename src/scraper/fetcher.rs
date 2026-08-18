use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

/// Create a shared HTTP client with reasonable defaults.
/// This should be created once and reused across all scrapers.
pub fn create_http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("RaceTUI/0.1.0")
        .build()
        .context("Failed to create HTTP client")
}

/// Fetch a URL and return the response body as a string.
pub async fn fetch_url(client: &Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {} for URL: {}", status, url);
    }

    response
        .text()
        .await
        .with_context(|| format!("Failed to read response body from: {}", url))
}

/// Fetch a URL and parse the response as JSON.
pub async fn fetch_json<T: serde::de::DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {} for URL: {}", status, url);
    }

    response
        .json::<T>()
        .await
        .with_context(|| format!("Failed to parse JSON from: {}", url))
}

/// Fetch a URL using a headless browser (for JavaScript-rendered pages).
/// Only available when compiled with the `headless-browser` feature.
#[cfg(feature = "headless-browser")]
pub async fn fetch_with_browser(url: &str) -> Result<String> {
    use chromiumoxide::Browser;
    use chromiumoxide::BrowserConfig;
    use futures::StreamExt;

    let (browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .build()
            .map_err(|e| anyhow::anyhow!("Browser config error: {}", e))?,
    )
    .await
    .context("Failed to launch headless browser")?;

    // The handler must be polled in the background
    tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    let page = browser.new_page(url).await.context("Failed to open page")?;

    // Wait for page to fully load
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let html = page.content().await.context("Failed to get page content")?;

    Ok(html)
}

#[cfg(not(feature = "headless-browser"))]
pub async fn fetch_with_browser(_url: &str) -> Result<String> {
    anyhow::bail!(
        "Headless browser support is not compiled in. \
         Build with: cargo build --features headless-browser"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_http_client() {
        let client = create_http_client();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_with_browser_fallback() {
        #[cfg(not(feature = "headless-browser"))]
        {
            let res = fetch_with_browser("https://example.com").await;
            assert!(res.is_err());
            assert!(res
                .unwrap_err()
                .to_string()
                .contains("Headless browser support is not compiled in"));
        }
    }
}
