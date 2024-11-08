//! Analytics collection.

use crate::Context;
use analytics_stub::Access;
use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use std::sync::Arc;

async fn insert_accesses(
    ctx: &Context,
    accesses: Vec<Access>,
) -> Result<(), tokio_postgres::Error> {
    for access in accesses {
        let geolocation = access.peer_addr.and_then(|ip| {
            let geolocation = ctx.geoip.lock().resolve(ip).ok()?;
            Some(serde_json::to_value(geolocation).unwrap())
        });
        let device = access.user_agent.as_ref().map(|ua| {
            let device = ctx.uaparser.lock().resolve(ua);
            serde_json::to_value(device).unwrap()
        });
        ctx.db.execute("INSERT INTO analytics (date, peer_addr, user_agent, referer, geolocation, device, method, uri) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
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
