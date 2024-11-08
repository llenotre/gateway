//! API for the gateway and various utilities for HTTP services.

use std::sync::OnceLock;

pub mod analytics;

/// Returns the content of the `robots.txt` file.
///
/// `host` is the hostname of the service.
pub fn robots(host: &str) -> &'static str {
    static ROBOTS: OnceLock<String> = OnceLock::new();
    ROBOTS.get_or_init(|| {
        format!(include_str!("robots.txt"), host)
    })
}
