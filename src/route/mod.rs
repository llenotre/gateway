//! The API's endpoints.

pub mod analytics;
pub mod newsletter;

use crate::Context;
use axum::{
	body::Body,
	extract::{State},
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::error;

/// Json representing the service's health.
#[derive(Serialize)]
pub struct Health<'s> {
	/// The service's status.
	status: &'s str,
	/// In case of error, the reason.
	reason: Option<String>,
}

pub async fn health(State(ctx): State<Arc<Context>>) -> Response {
	let res = ctx.db.execute("SELECT 1 + 1", &[]).await;
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

/// GitHub avatar proxy endpoint, to protect users from data collection (GDPR).
pub async fn avatar() -> Response {
	let client = reqwest::Client::new();
	let res = client.get("https://github.com/llenotre.png").send().await;
	let response = match res {
		Ok(r) => r,
		Err(error) => {
			error!(%error, "could not get avatar from Github");
			return (StatusCode::BAD_GATEWAY, "bad gateway").into_response();
		}
	};
	let status = StatusCode::from_u16(response.status().as_u16()).unwrap();
	let mut builder = Response::builder()
		.status(status)
		.header("Cache-Control", "max-age=604800");
	if let Some(content_type) = response.headers().get("Content-Type") {
		builder = builder.header("Content-Type", content_type);
	}
	builder
		.body(Body::from_stream(response.bytes_stream()))
		.unwrap()
}
