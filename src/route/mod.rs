use crate::Context;
use analytics_stub::Access;
use axum::body::Body;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use reqwest::StatusCode;
use serde::Serialize;
use std::sync::Arc;

/// Json representing the service's health.
#[derive(Serialize)]
pub struct Health<'s> {
    /// The service's status.
    status: &'s str,
    /// In case of error, the reason.
    reason: Option<String>,
}

pub async fn health(State(ctx): State<Arc<Context>>) -> Response {
    let res = {
        let db = ctx.db.read().await;
        db.execute("SELECT 1 + 1", &[]).await
    };
    match res {
        Ok(_) => Json(Health {
            status: "OK",
            reason: None,
        })
        .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Health {
                status: "KO",
                reason: Some(error.to_string()),
            }),
        )
            .into_response(),
    }
}

async fn insert_accesses(
    ctx: &Context,
    accesses: Vec<Access>,
) -> Result<(), tokio_postgres::Error> {
    let db = ctx.db.read().await;
    for access in accesses {
        let geolocation = access.peer_addr.and_then(|ip| {
            let geolocation = ctx.geoip.lock().resolve(ip).ok()?;
            Some(serde_json::to_value(geolocation).unwrap())
        });
        let device = access.user_agent.as_ref().map(|ua| {
            let device = ctx.uaparser.lock().resolve(ua);
            serde_json::to_value(device).unwrap()
        });
        db.execute("INSERT INTO analytics (date, peer_addr, user_agent, referer, geolocation, device, method, uri) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
         &[
             &access.date,
             &access.peer_addr,
             &access.user_agent,
             &access.referer,
             &geolocation,
             &device,
             &access.method,
             &access.uri,
         ]).await?;
    }
    Ok(())
}

pub async fn access(
    State(ctx): State<Arc<Context>>,
    Json(accesses): Json<Vec<Access>>,
) -> Response {
    let res = insert_accesses(&ctx, accesses).await;
    let builder = Response::builder();
    match res {
        Ok(_) => builder.status(StatusCode::OK).body(Body::empty()).unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .unwrap(),
    }
}
