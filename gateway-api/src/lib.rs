//! API for the gateway and various utilities for HTTP services.

use serde::Deserialize;
use std::sync::OnceLock;

pub mod analytics;
pub mod util;

/// Configuration for the API.
#[derive(Deserialize)]
pub struct Config {
	/// URL to the gateway.
	pub url: String,
	/// The property's UUID.
	pub property: String,
	/// The property's secret.
	pub secret: String,

	/// The current service's hostname.
	pub host: String,
}

impl Config {
	/// Returns the configuration from the current environment.
	///
	/// If the configuration is incorrect, the function panics.
	pub fn get() -> &'static Self {
		static CONFIG: OnceLock<Config> = OnceLock::new();
		CONFIG.get_or_init(|| {
			envy::prefixed("GATEWAY_")
				.from_env()
				.expect("configuration")
		})
	}
}

/// Returns the content of the `robots.txt` file.
pub fn robots() -> &'static str {
	static ROBOTS: OnceLock<String> = OnceLock::new();
	ROBOTS.get_or_init(|| {
		let config = Config::get();
		format!(include_str!("robots.txt"), config.host)
	})
}
