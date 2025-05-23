use crate::util::Renewable;
use anyhow::Result;
use maxminddb::MaxMindDbError;
use serde::Serialize;
use std::net::IpAddr;

/// A user's geolocation.
#[derive(Serialize)]
pub struct UserGeolocation {
	city: Option<String>,
	continent: Option<String>,
	country: Option<String>,

	latitude: Option<f64>,
	longitude: Option<f64>,
	accuracy_radius: Option<u16>,
	time_zone: Option<String>,
}

/// Determines the location of a user from its IP address.
pub struct GeoIP(maxminddb::Reader<Vec<u8>>);

impl Renewable for GeoIP {
	fn new(data: Vec<u8>) -> Result<Self> {
		Ok(Self(maxminddb::Reader::from_source(data)?))
	}
}

impl GeoIP {
	pub fn resolve(&self, addr: IpAddr) -> Result<Option<UserGeolocation>, MaxMindDbError> {
		let geolocation = self
			.0
			.lookup::<maxminddb::geoip2::City>(addr)?
			.map(|geolocation| UserGeolocation {
				city: geolocation
					.city
					.and_then(|c| c.names)
					.as_ref()
					.and_then(|n| n.get("en").or_else(|| n.values().next()))
					.map(|s| (*s).to_owned()),
				continent: geolocation
					.continent
					.and_then(|c| c.code)
					.map(str::to_owned),
				country: geolocation
					.country
					.and_then(|c| c.iso_code)
					.map(str::to_owned),

				latitude: geolocation.location.as_ref().and_then(|c| c.latitude),
				longitude: geolocation.location.as_ref().and_then(|c| c.longitude),
				accuracy_radius: geolocation
					.location
					.as_ref()
					.and_then(|c| c.accuracy_radius),
				time_zone: geolocation
					.location
					.as_ref()
					.and_then(|c| c.time_zone)
					.map(str::to_owned),
			});
		Ok(geolocation)
	}
}
