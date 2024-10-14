#![feature(duration_constructors)]

mod route;
mod uaparser;

use crate::route::{access, health};
use crate::uaparser::UaParser;
use axum::routing::{get, put};
use axum::Router;
use serde::Deserialize;
use std::io;
use std::process::exit;
use std::sync::Arc;
use tokio::select;
use tokio::sync::RwLock;
use tokio_postgres::NoTls;
use tracing::{error, info};

#[derive(Deserialize)]
struct Config {
    /// The port the server listens to.
    pub port: u32,
    /// The connection string to the database.
    pub db: String,
}

struct Context {
    db: RwLock<tokio_postgres::Client>,
    uaparser: UaParser,
}

#[tokio::main]
async fn main() -> io::Result<()> {
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
    let ctx = Arc::new(Context {
        db: RwLock::new(client),
        uaparser: UaParser::new().await.expect("UaParser failure"),
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
        res = connection => res.expect("Database failure"), // TODO must reconnect instead. crashing will cause the lost of pending accesses to be inserted
    }
    Ok(())
}
