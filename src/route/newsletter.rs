//! Newsletter endpoints.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use crate::Context;
use crate::service::newsletter::insert_subscriber;

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
pub async fn subscribe(State(ctx): State<Context>, Json(payload): Json<SubscribePayload>) {
    let _res = {
        let db = ctx.db.read().await;
        insert_subscriber(&db, &payload.email).await
    };
    todo!()
}

/// Endpoint to unsubscribe from a newsletter.
pub async fn unsubscribe(Json(payload): Json<UnsubscribePayload>) {
    todo!()
}
