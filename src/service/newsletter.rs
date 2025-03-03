//! Newsletter logic.

use crate::util::PgResult;
use chrono::Utc;

/// Insert a new email in the newsletter subscribers list.
pub async fn insert_subscriber(db: &tokio_postgres::Client, email: &str) -> PgResult<()> {
	let now = Utc::now().naive_utc();
	db.execute(
		"INSERT INTO newsletter_subscriber (email, subscribe_date)\
			VALUES ($1, $2) ON CONFLICT DO NOTHING",
		&[&email, &now],
	)
	.await?;
	Ok(())
}

/// Unsubscribes a user from the newsletter using the given email token.
///
/// On success, the function returns `true`. If no associated token or email is found, the
/// function returns `false`.
pub async fn unsubscribe_from_token(
	db: &tokio_postgres::Client,
	token: &String,
) -> PgResult<bool> {
	let now = Utc::now();
	let n = db
		.execute(
			r#"UPDATE newsletter_subscriber SET email = NULL unsubscribe_date = $1 unsubscribe_token = $2
				WHERE email = (SELECT recipient FROM newsletter_email WHERE token = $2)"#,
			&[&now, token],
		)
		.await?;
	Ok(n > 0)
}
