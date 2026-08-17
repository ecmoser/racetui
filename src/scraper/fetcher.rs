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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_http_client() {
        let client = create_http_client();
        assert!(client.is_ok());
    }
}
