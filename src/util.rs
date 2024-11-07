//! Utilities.

use anyhow::{bail, Result};
use tracing::trace;

/// Fetches a file from the given URL and returns its content.
pub async fn fetch(url: &str) -> Result<Vec<u8>> {
    let response = reqwest::get(url).await?;
    let status = response.status();
    let body = response.bytes().await?.to_vec();
    if status.is_success() {
        Ok(body)
    } else {
        trace!(status = status.as_u16(), ?body, "could not fetch from URL");
        bail!("could not fetch from URL (status {})", status.as_u16())
    }
}
