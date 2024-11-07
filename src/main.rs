//! Analytics aggregator.

mod geoip;
mod route;
mod uaparser;
mod util;

use crate::geoip::GeoIP;
use crate::route::{access, health};
use crate::uaparser::UaParser;
use crate::util::Renewer;
use axum::routing::{get, put};
use axum::Router;
use serde::Deserialize;
use std::io;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio_postgres::NoTls;
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
    let (client, connection) = tokio_postgres::connect(&config.db, NoTls)
        .await
        .unwrap_or_else(|error| {
            error!(%error, "postgres: connection");
            exit(1);
        });
    info!("prepare context");
    let ctx = Arc::new(Context {
        db: RwLock::new(client),
        uaparser: Renewer::new(config.uaparser_url, None)
            .await
            .expect("UaParser failure"),
        geoip: Renewer::new(
            config.geoip_url,
            Some((config.geoip_user, config.geoip_password)),
        )
        .await
        .expect("GeoIP failure"),
    });
    let ctx_ = ctx.clone();
    let renew_task = tokio::spawn(async {
        // 1 day
        let mut interval = interval(Duration::from_secs(60 * 60 * 24));
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
    info!("start http server");
    let app = Router::new()
        .route("/health", get(health))
        .route("/access", put(access))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    select! {
        res = axum::serve(listener, app) => res.expect("HTTP failure"),
        res = connection => res.expect("Database failure"),
        _ = renew_task => panic!("Resource renew failure"),
    }
    Ok(())
}
