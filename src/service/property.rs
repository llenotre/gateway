//! Property logic.

use crate::util::PgResult;
use uuid::Uuid;

/// Returns the property ID for the given secret.
///
/// If the property does not exist, the function returns `None`.
pub async fn from_secret(
	db: &tokio_postgres::Client,
	uuid: &String,
	secret: &String,
) -> PgResult<Option<Uuid>> {
	// TODO hash secret
	let res = db
		.query_opt(
			"SELECT uuid FROM property WHERE uuid = $1 AND secret = $2",
			&[uuid, secret],
		)
		.await?;
	let Some(row) = res else {
		return Ok(None);
	};
	Ok(Some(row.get::<_, Uuid>(0)))
}
