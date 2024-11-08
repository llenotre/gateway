//! Newsletter endpoints.

use crate::{
	service::newsletter::{insert_subscriber, unsubscribe_from_token},
	util::validate_email,
	Context,
};
use axum::{
	body::Body,
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::error;

/// Payload of request to register a newsletter subscriber.
#[derive(Deserialize)]
pub struct SubscribePayload {
	/// The email of the subscriber.
	email: String,
}

/// Payload of request to unregister a newsletter subscriber.
#[derive(Deserialize)]
pub struct UnsubscribePayload {
	/// The unsubscribe token.
	token: String,
}

/// Endpoint to subscribe to a newsletter.
pub async fn subscribe(
	State(ctx): State<Arc<Context>>,
	Json(payload): Json<SubscribePayload>,
) -> Response {
	if !validate_email(&payload.email) {
		return (StatusCode::BAD_REQUEST, "invalid email address").into_response();
	}
	let res = insert_subscriber(&ctx.db, &payload.email).await;
	match res {
		Ok(_) => Response::new(Body::empty()),
		Err(error) => {
			error!(%error, "could not add newsletter subscriber");
			(StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
		}
	}
}

/// Endpoint to unsubscribe from a newsletter.
pub async fn unsubscribe(
	State(ctx): State<Arc<Context>>,
	Json(payload): Json<UnsubscribePayload>,
) -> Response {
	let res = unsubscribe_from_token(&ctx.db, &payload.token).await;
	match res {
		Ok(_) => Response::new(Body::empty()),
		Err(error) => {
			error!(%error, "could not remove newsletter subscriber");
			(StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
		}
	}
}
