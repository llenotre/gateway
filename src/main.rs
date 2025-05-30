//! Data collection and authentication service.

#![feature(duration_constructors)]
#![feature(duration_constructors_lite)]

mod route;
mod service;
mod util;

use crate::{
	service::{geoip::GeoIP, uaparser::UaParser},
	util::{RenewableInfo, Renewer},
};
use axum::{
	Router,
	routing::{get, post, put},
};
use chrono::Utc;
use gateway_api::log::LogLayer;
use serde::Deserialize;
use std::{io, net::SocketAddr, process::exit, sync::Arc, time::Duration};
use tokio::{select, sync::RwLock, time::interval};
use tokio_postgres::NoTls;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
	cors,
	cors::{AllowHeaders, CorsLayer},
};
use tracing::{error, info, warn};

#[derive(Deserialize)]
struct Config {
	/// The port the server listens to.
	pub port: u32,
	/// The connection string to the database.
	pub db: String,
	/// The URL to fetch uaparser data.
	pub uaparser_url: String,
	/// The URL to fetch geoip data.
	pub geoip_url: String,
	/// The geoip username (account ID).
	pub geoip_user: String,
	/// The geoip password (license key).
	pub geoip_password: String,
}

struct Context {
	db: RwLock<tokio_postgres::Client>,
	uaparser: Renewer<UaParser>,
	geoip: Renewer<GeoIP>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
	tracing_subscriber::fmt::init();
	let config = envy::from_env::<Config>().unwrap_or_else(|error| {
		error!(%error, "invalid configuration");
		exit(1);
	});
	info!("connect to database");
	// TODO tls
	let (client, mut connection) = tokio_postgres::connect(&config.db, NoTls)
		.await
		.unwrap_or_else(|error| {
			error!(%error, "postgres: connection");
			exit(1);
		});
	info!("prepare context");
	let ctx = Arc::new(Context {
		db: RwLock::new(client),
		uaparser: Renewer::new(RenewableInfo {
			url: config.uaparser_url,
			auth: None,
			compressed: false,
		})
		.await
		.expect("UaParser failure"),
		geoip: Renewer::new(RenewableInfo {
			url: config.geoip_url,
			auth: None,
			compressed: false,
		})
		// TODO find a solution for the rate limit
		/*geoip: Renewer::new(RenewableInfo {
			url: config.geoip_url,
			auth: Some((config.geoip_user, config.geoip_password)),
			compressed: true,
		})*/
		.await
		.expect("GeoIP failure"),
	});
	info!("start background tasks");
	// Setup postgres reconnection task
	let ctx_ = ctx.clone();
	let db_task = tokio::spawn(async move {
		let ctx = ctx_;
		'a: loop {
			if let Err(error) = connection.await {
				error!(%error, "database connection error");
			}
			// Try to reconnect
			const MAX_ATTEMPTS: usize = 10;
			let mut interval = interval(Duration::from_secs(10));
			for attempts in 1..=MAX_ATTEMPTS {
				interval.tick().await;
				info!("attempting database reconnection ({attempts}/{MAX_ATTEMPTS})");
				let res = tokio_postgres::connect(&config.db, NoTls).await;
				match res {
					Ok((cl, conn)) => {
						info!("database reconnection success");
						*ctx.db.write().await = cl;
						connection = conn;
						continue 'a;
					}
					Err(error) => error!(%error, "database reconnection failure"),
				}
			}
			// stop attempting and crash
			break;
		}
	});
	// Setup databases renew task
	let ctx_ = ctx.clone();
	let renew_task = tokio::spawn(async {
		let mut interval = interval(Duration::from_days(1));
		let ctx = ctx_;
		loop {
			interval.tick().await;
			if let Err(error) = ctx.uaparser.renew().await {
				warn!(%error, "could not renew UaParser");
			}
			if let Err(error) = ctx.geoip.renew().await {
				warn!(%error, "could not renew GeoIP");
			}
		}
	});
	// Setup analytics anonymization task
	let ctx_ = ctx.clone();
	let anonymize_task = tokio::spawn(async {
		let mut interval = interval(Duration::from_hours(1));
		let ctx = ctx_;
		loop {
			interval.tick().await;
			// The end of the date range in which entries are going to be anonymized
			let end = Utc::now().naive_utc() - Duration::from_days(365);
			let db = ctx.db.read().await;
			let res = db.execute(
                "UPDATE analytics SET peer_addr = NULL, user_agent = NULL WHERE date <= $1 AND (peer_addr IS NOT NULL OR user_agent IS NOT NULL)",
                &[&end],
            )
                .await;
			if let Err(error) = res {
				warn!(%error, "could not anonymize analytics");
			}
		}
	});
	// Setup rate limiting
	let governor_conf = Arc::new(
		GovernorConfigBuilder::default()
			.per_second(1)
			.burst_size(5)
			.finish()
			.unwrap(),
	);
	let governor_limiter = governor_conf.limiter().clone();
	let rate_limit_task = tokio::spawn(async move {
		let mut interval = interval(Duration::from_mins(1));
		loop {
			interval.tick().await;
			governor_limiter.retain_recent();
		}
	});
	info!("start http server");
	let app = Router::new()
		.route("/health", get(route::health))
		.route("/access", put(route::analytics::access))
		.route("/avatar", get(route::avatar))
		.route("/newsletter/subscribe", post(route::newsletter::subscribe))
		.route(
			"/newsletter/unsubscribe",
			post(route::newsletter::unsubscribe),
		)
		.layer(
			CorsLayer::new()
				.allow_origin(cors::Any)
				.allow_headers(AllowHeaders::any()),
		)
		.layer(GovernorLayer {
			config: governor_conf,
		})
		.layer(LogLayer)
		.with_state(ctx)
		.into_make_service_with_connect_info::<SocketAddr>();
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
	select! {
		res = axum::serve(listener, app) => res.expect("HTTP failure"),
		_ = db_task => panic!("Database task failure"),
		_ = renew_task => panic!("Resource renew task failure"),
		_ = anonymize_task => panic!("Anonymization task failure"),
		_ = rate_limit_task => panic!("Rate limiting task failure"),
	}
	Ok(())
}
