//! The API's endpoints.

pub mod analytics;
pub mod newsletter;

use crate::Context;
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
