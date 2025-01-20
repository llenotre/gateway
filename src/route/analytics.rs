//! Analytics collection.

use crate::{service::property, Context};
use axum::{
	body::Body,
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use axum_auth::AuthBasic;
use gateway_api::analytics::Access;
use std::sync::Arc;
use tracing::{error, warn};
use uuid::Uuid;

async fn insert_accesses(
	ctx: &Context,
	property: &Uuid,
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
		db.execute("INSERT INTO analytics (property, date, peer_addr, user_agent, referer, geolocation, device, method, uri) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT DO NOTHING",
		&[
			property,
			&access.date.naive_utc(),
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
	AuthBasic((uuid, secret)): AuthBasic,
	Json(accesses): Json<Vec<Access>>,
) -> Response {
	let (Ok(uuid), Some(Ok(secret))) = (
		Uuid::parse_str(&uuid),
		secret.as_deref().map(Uuid::parse_str),
	) else {
		warn!("authentication failure");
		return (StatusCode::UNAUTHORIZED, Body::empty()).into_response();
	};
	let db = ctx.db.read().await;
	let res = property::check_auth(&db, &uuid, &secret).await;
	match res {
		Ok(true) => {}
		Ok(false) => {
			warn!("authentication failure");
			return (StatusCode::UNAUTHORIZED, Body::empty()).into_response();
		}
		Err(error) => {
			error!(%error, "could not check secret");
			return (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response();
		}
	}
	let res = insert_accesses(&ctx, &uuid, accesses).await;
	match res {
		Ok(_) => Response::new(Body::empty()),
		Err(error) => {
			error!(%error, "could not insert accesses");
			(StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response()
		}
	}
}
