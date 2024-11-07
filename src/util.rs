//! Utilities.

use anyhow::{bail, Result};
use std::sync::{RwLock, RwLockReadGuard};
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

/// A resource that can be periodically renewed.
pub trait Renewable: Sized {
    /// Creates a new instance from the given data.
    fn new(data: Vec<u8>) -> Result<Self>;
}

/// Wrapper allowing to renew the underlying resource.
pub struct Renewer<T: Renewable> {
    /// The URL to fetch the resource's data from.
    url: String,
    /// The resource.
    inner: RwLock<T>,
}

impl<T: Renewable> Renewer<T> {
    /// Creates a new instance.
    ///
    /// The renewer fetches the required data from the given `url`.
    pub async fn new(url: String) -> Result<Self> {
        let data = fetch(&url).await?;
        let inner = T::new(data)?;
        Ok(Self {
            url,
            inner: RwLock::new(inner),
        })
    }

    /// Renew the resource.
    pub async fn renew(&self) -> Result<()> {
        let data = fetch(&self.url).await?;
        let inner = T::new(data)?;
        let mut guard = self.inner.write().unwrap();
        *guard = inner;
        Ok(())
    }

    /// Locks the inner value and returns the guard.
    pub fn lock(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read().unwrap()
    }
}
