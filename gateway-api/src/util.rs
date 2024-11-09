//! Utilities.

use std::net::IpAddr;
use axum::extract::Request;
use axum_client_ip::InsecureClientIp;
use futures::executor::block_on;
use axum::extract::FromRequestParts;

/// Extracts the peer address from a request.
///
/// If the address cannot be extracted, the returned address is `None`.
pub fn extract_peer_addr(request: Request) -> (Request, Option<IpAddr>) {
	let (mut parts, body) = request.into_parts();
	// According to the crate's documentation, `InsecureClientIp` is fine for geolocation
	let peer_addr = block_on(InsecureClientIp::from_request_parts(&mut parts, &()))
		.ok()
		.map(|addr| addr.0);
	let request = Request::from_parts(parts, body);
	(request, peer_addr)
}

/// Date serialization/deserialization.
pub mod date_format {
	use chrono::{DateTime, NaiveDateTime, Utc};
	use serde::{Deserialize, Deserializer, Serializer};

	pub const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

	pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let str = date.format(FORMAT).to_string();
		serializer.serialize_str(&str)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let dt = NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
		Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
	}
}
