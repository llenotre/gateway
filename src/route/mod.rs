use std::sync::Arc;
use axum::body::Body;
use axum::extract::State;
use axum::Json;
use axum::response::{Response};
use reqwest::StatusCode;
use serde::Deserialize;
use crate::Context;

pub async fn health(State(ctx): State<Arc<Context>>) -> Response {
    let res = {
        let db = ctx.db.read().await;
        db.execute("SELECT 1 + 1", &[]).await
    };
    let builder = Response::builder();
    match res {
        Ok(2) => builder
            .status(StatusCode::OK)
            .body(Body::from("OK"))
            .unwrap(),
        _ => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("KO"))
            .unwrap(),
    }
}

#[derive(Deserialize)]
pub struct Access {
    /// UTC timestamp in seconds.
    date: i64,
    peer_addr: Option<String>,
    user_agent: Option<String>,
    referer: Option<String>,
    method: String,
    uri: String,
}

pub async fn access(State(ctx): State<Arc<Context>>, Json(accesses): Json<Vec<Access>>) -> &'static str {
    let db = ctx.db.read().await;
    // TODO batch insert
    for access in accesses {
        let geolocation = ""; // TODO
        let device = ""; // TODO
        let res = db.execute("INSERT INTO analytics (date, peer_addr, user_agent, referer, geolocation, device, method, uri) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
         &[
             &access.date,
             &access.peer_addr,
             &access.user_agent,
             &access.referer,
             &geolocation,
             &device,
             &access.method,
             &access.uri,
         ]).await;
        // TODO on error, store in pool and retry later
    }
    ""
}
