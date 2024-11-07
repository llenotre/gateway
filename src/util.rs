//! Utilities.

use anyhow::{bail, Result};
use std::sync::{RwLock, RwLockReadGuard};
use tracing::trace;

/// Fetches a file from the given URL and returns its content.
pub async fn fetch(url: &str, auth: Option<(&str, &str)>) -> Result<Vec<u8>> {
    trace!(url, "fetch resource");
    let client = reqwest::Client::new();
    let mut request = client.get(url);
    if let Some(auth) = auth {
        request = request.basic_auth(auth.0, Some(auth.1));
    }
    let response = request.send().await?;
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
    /// Optional basic auth.
    auth: Option<(String, String)>,

    /// The resource.
    inner: RwLock<T>,
}

impl<T: Renewable> Renewer<T> {
    /// Creates a new instance.
    ///
    /// The renewer fetches the required data from the given `url`.
    pub async fn new(url: String, auth: Option<(String, String)>) -> Result<Self> {
        let auth_deref = auth.as_ref().map(|(u, p)| (u.as_str(), p.as_str()));
        let data = fetch(&url, auth_deref).await?;
        let inner = T::new(data)?;
        Ok(Self {
            url,
            auth,

            inner: RwLock::new(inner),
        })
    }

    /// Renew the resource.
    pub async fn renew(&self) -> Result<()> {
        let auth = self.auth.as_ref().map(|(u, p)| (u.as_str(), p.as_str()));
        let data = fetch(&self.url, auth).await?;
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
