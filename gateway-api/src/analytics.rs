//! Analytics management.

use crate::{util, Config};
use axum::{
	extract::{Request},
	http::header::{REFERER, USER_AGENT},
	response::Response,
};
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::{
	net::IpAddr,
	sync::Arc,
	task::{Context, Poll},
	time::Duration,
};
use tokio::{
	select,
	sync::mpsc::{unbounded_channel, UnboundedSender},
	time::interval,
};
use tower::{Layer, Service};
use tracing::{error, info};
use crate::util::extract_peer_addr;

const FLUSH_THRESHOLD: usize = 1024;

/// An access log, emitted when accessing an endpoint.
#[derive(Clone, Deserialize, Serialize)]
pub struct Access {
	#[serde(with = "util::date_format")]
	pub date: DateTime<Utc>,
	pub peer_addr: Option<IpAddr>,
	pub user_agent: Option<String>,
	pub referer: Option<String>,
	pub method: String,
	pub uri: String,
}

/// A pool containing accesses to be flushed.
pub struct AccessPool {
	sender: UnboundedSender<Access>,
}

impl AccessPool {
	/// Creates a new pool
	fn new() -> Self {
		let (sender, mut receiver) = unbounded_channel::<Access>();
		tokio::spawn(async move {
			let mut interval = interval(Duration::from_secs(10));
			let mut buf = Vec::with_capacity(FLUSH_THRESHOLD);
			loop {
				let max = FLUSH_THRESHOLD.saturating_sub(buf.len());
				select! {
					_ = interval.tick() => Self::flush(&mut buf).await,
					len = receiver.recv_many(&mut buf, max) => {
						// If the queue is closed, stop
						if len == 0 {
							break;
						}
						// If the buffer is full, flush
						if buf.len() >= FLUSH_THRESHOLD {
							Self::flush(&mut buf).await;
						}
					}
				}
			}
			drop(receiver);
			// Attempt to flush remaining accesses in buffer before stopping
			while !buf.is_empty() {
				Self::flush(&mut buf).await;
				interval.tick().await;
			}
		});
		Self {
			sender,
		}
	}

	/// Pushes new access to the pool.
	async fn push(&self, access: Access) {
		// If the other side is closed, that means the server is stopping anyway
		let _ = self.sender.send(access);
	}

	/// Flushes the pool's content, clearing it.
	///
	/// If the pool could not be flushed, its content is kept for a future retry.
	async fn flush(pool: &mut Vec<Access>) {
		if pool.is_empty() {
			return;
		}
		info!(count = pool.len(), "attempt to flush accesses");
		let config = Config::get();
		let url = format!("{}/access", config.gateway_url);
		// HTTP request to push accesses
		let client = reqwest::Client::new();
		let res = client
			.put(&url)
			.basic_auth(&config.gateway_property, Some(&config.gateway_secret))
			.json(pool)
			.send()
			.await;
		let response = match res {
			Ok(response) => response,
			Err(error) => {
				error!(url, %error, "access: HTTP call failure");
				return;
			}
		};
		let status = response.status();
		if !status.is_success() {
			error!(url, %status, "access: HTTP call failure");
			return;
		}
		// Success: clear pool
		info!("clear accesses pool");
		pool.clear();
	}
}

/// Analytics collection layer.
///
/// **Note**: This layer requires connection information. The following call is required on the
/// router:
///
/// ```
/// into_make_service_with_connect_info::<SocketAddr>()
/// ```
#[derive(Clone)]
pub struct AnalyticsLayer {
	pool: Arc<AccessPool>,
}

impl Default for AnalyticsLayer {
	fn default() -> Self {
		Self {
			pool: Arc::new(AccessPool::new()),
		}
	}
}

impl<S> Layer<S> for AnalyticsLayer {
	type Service = AnalyticsMiddleware<S>;

	fn layer(&self, inner: S) -> Self::Service {
		AnalyticsMiddleware {
			inner,
			pool: self.pool.clone(),
		}
	}
}

/// Middleware collecting analytics data.
#[derive(Clone)]
pub struct AnalyticsMiddleware<S> {
	inner: S,
	pool: Arc<AccessPool>,
}

impl<S> Service<Request> for AnalyticsMiddleware<S>
where
	S: Service<Request, Response = Response> + Send + 'static,
	S::Future: Send + 'static,
{
	type Error = S::Error;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
	type Response = S::Response;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, request: Request) -> Self::Future {
		let (request, peer_addr) = extract_peer_addr(request);
		let access = Access {
			date: Utc::now(),
			peer_addr,
			user_agent: request
				.headers()
				.get(USER_AGENT)
				.and_then(|ua| ua.to_str().ok())
				.map(str::to_owned),
			referer: request
				.headers()
				.get(REFERER)
				.and_then(|ua| ua.to_str().ok())
				.map(str::to_owned),
			method: request.method().to_string(),
			uri: request.uri().to_string(),
		};
		let pool = self.pool.clone();
		let future = self.inner.call(request);
		Box::pin(async move {
			pool.push(access).await;
			let response: Response = future.await?;
			Ok(response)
		})
	}
}
