use std::net::IpAddr;
use std::time::{Duration, Instant};
use anyhow::Result;
use maxminddb::MaxMindDBError;
use serde::Serialize;
use crate::Config;
use crate::util::fetch;

const RENEW_INTERVAL: Duration = Duration::from_days(1);

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
pub struct GeoIP {
    inner: maxminddb::Reader<Vec<u8>>,
    last_renew: Instant,
}

impl GeoIP {
    pub async fn new(config: &Config) -> Result<Self> {
        let data = fetch(&config.geoip_url).await?;
        let inner = maxminddb::Reader::from_source(data)?;
        Ok(Self {
            inner,
            last_renew: Instant::now(),
        })
    }

    pub fn resolve(&self, addr: IpAddr) -> Result<UserGeolocation, MaxMindDBError> {
        let geolocation: maxminddb::geoip2::City = self.inner.lookup(addr)?;
        Ok(UserGeolocation {
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
        })
    }
}
