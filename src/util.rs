//! Utilities.

/// Fetches a file from the given URL and returns its content.
pub async fn fetch(url: &str) -> reqwest::Result<Vec<u8>> {
    Ok(reqwest::get(url).await?.bytes().await?.to_vec())
}
