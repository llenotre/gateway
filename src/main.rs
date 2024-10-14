mod route;

use std::{io};
use std::process::exit;
use std::sync::{Arc};
use axum::Router;
use axum::routing::{get, put};
use serde::Deserialize;
use tokio::{select};
use tokio::sync::RwLock;
use tokio_postgres::NoTls;
use tracing::{error, info};
use crate::route::{access, health};

#[derive(Deserialize)]
struct Config {
    /// The port the server listens to.
    pub port: u32,
    /// The connection string to the database.
    pub db: String,
}

struct Context {
    db: RwLock<tokio_postgres::Client>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = envy::from_env::<Config>()
        .unwrap_or_else(|error| {
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
    let ctx = Arc::new(Context {
        db: RwLock::new(client),
    });
    // TODO handle DB reconnect
    info!("start http server");
    let app = Router::new()
        .route("/health", get(health))
        .route("/access", put(access))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    select! {
        res = axum::serve(listener, app) => res.expect("HTTP failure"),
        res = connection => res.expect("Database failure"),
    }
    Ok(())
}
