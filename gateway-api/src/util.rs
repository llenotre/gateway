//! Utilities.

/// Date serialization/deserialization.
pub mod date_format {
	use chrono::{DateTime, NaiveDateTime, Utc};
	use serde::{Deserialize, Deserializer, Serializer};

	const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

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