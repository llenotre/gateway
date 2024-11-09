//! API for the gateway and various utilities for HTTP services.

pub mod analytics;
pub mod log;
pub mod util;

use serde::Deserialize;
use std::sync::OnceLock;

/// Configuration for the API.
#[derive(Deserialize)]
pub struct Config {
	/// URL to the gateway.
	pub gateway_url: String,
	/// The property's UUID.
	pub gateway_property: String,
	/// The property's secret.
	pub gateway_secret: String,

	/// The current service's hostname.
	pub host: String,
}

impl Config {
	/// Returns the configuration from the current environment.
	///
	/// If the configuration is incorrect, the function panics.
	pub fn get() -> &'static Self {
		static CONFIG: OnceLock<Config> = OnceLock::new();
		CONFIG.get_or_init(|| envy::from_env().expect("configuration"))
	}
}

/// Endpoint for the `robots.txt` file.
pub async fn robots() -> &'static str {
	static ROBOTS: OnceLock<String> = OnceLock::new();
	ROBOTS.get_or_init(|| {
		let config = Config::get();
		format!(include_str!("robots.txt"), config.host)
	})
}
