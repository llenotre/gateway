//! Property logic.

use crate::util::PgResult;
use uuid::Uuid;

/// Checks authentication for a property.
pub async fn check_auth(
	db: &tokio_postgres::Client,
	uuid: &Uuid,
	secret: &Uuid,
) -> PgResult<bool> {
	// TODO hash secret?
	let row = db
		.query_opt(
			"SELECT uuid FROM property WHERE uuid = $1 AND secret = $2",
			&[uuid, secret],
		)
		.await?;
	Ok(row.is_some())
}
