//! Utilities.

use anyhow::{bail, Result};
use flate2::read::GzDecoder;
use std::io::Read;
use std::sync::{OnceLock, RwLock, RwLockReadGuard};
use tracing::trace;
use regex::Regex;

/// Result with PostgreSQL error.
pub type PgResult<T> = std::result::Result<T, tokio_postgres::Error>;

/// Tells whether the given email is valid.
pub fn validate_email(email: &str) -> bool {
    static EMAIL_VALIDATION: OnceLock<Regex> = OnceLock::new();
    let regex = EMAIL_VALIDATION.get_or_init(|| {
        Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$").unwrap()
    });
    regex.is_match(email)
}

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

/// Information allowing to retrieve a resource.
pub struct RenewableInfo {
    /// The URL to fetch the resource's data from.
    pub url: String,
    /// Optional basic auth.
    pub auth: Option<(String, String)>,
    /// Tells whether the downloaded data is compressed (with gzip).
    pub compressed: bool,
}

/// Wrapper allowing to renew the underlying resource.
pub struct Renewer<T: Renewable> {
    info: RenewableInfo,
    inner: RwLock<T>,
}

impl<T: Renewable> Renewer<T> {
    async fn renew_impl(info: &RenewableInfo) -> Result<T> {
        let auth = info.auth.as_ref().map(|(u, p)| (u.as_str(), p.as_str()));
        let mut data = fetch(&info.url, auth).await?;
        if info.compressed {
            let mut decoder = GzDecoder::new(data.as_slice());
            let mut buf = vec![];
            decoder.read_to_end(&mut buf)?;
            data = buf;
        }
        T::new(data)
    }

    /// Creates a new instance.
    ///
    /// The renewer fetches the required data from the given `url`.
    pub async fn new(info: RenewableInfo) -> Result<Self> {
        let inner = Self::renew_impl(&info).await?;
        Ok(Self {
            info,
            inner: RwLock::new(inner),
        })
    }

    /// Renew the resource.
    pub async fn renew(&self) -> Result<()> {
        let inner = Self::renew_impl(&self.info).await?;
        let mut guard = self.inner.write().unwrap();
        *guard = inner;
        Ok(())
    }

    /// Locks the inner value and returns the guard.
    pub fn lock(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read().unwrap()
    }
}
