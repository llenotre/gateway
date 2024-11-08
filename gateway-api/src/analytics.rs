//! Analytics management.

use axum::{
    extract::{FromRequestParts, Request},
    http::header::{REFERER, USER_AGENT},
    response::Response,
};
use axum_client_ip::InsecureClientIp;
use chrono::Utc;
use futures::executor::block_on;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::{
    env,
    net::IpAddr,
    sync::LazyLock,
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

const FLUSH_THRESHOLD: usize = 1024;

/// Access pool configuration.
struct AccessPoolConfig {
    url: String,
    property: String,
    secret: String,
}

impl AccessPoolConfig {
    /// Returns the configuration from the current environment.
    ///
    /// If the environment is invalid, the function panics.
    fn from_env() -> Self {
        Self {
            url: env::var("ANALYTICS_URL").expect("ANALYTICS_URL must be set"),
            property: env::var("ANALYTICS_PROPERTY").expect("ANALYTICS_PROPERTY must be set"),
            secret: env::var("ANALYTICS_SECRET").expect("ANALYTICS_SECRET must be set"),
        }
    }
}

/// An access log, emitted when accessing an endpoint.
#[derive(Clone, Deserialize, Serialize)]
pub struct Access {
    /// UTC timestamp in seconds.
    pub date: i64,
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
        let config = AccessPoolConfig::from_env();
        let (sender, mut receiver) = unbounded_channel::<Access>();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            let mut buf = Vec::with_capacity(FLUSH_THRESHOLD);
            loop {
                let max = FLUSH_THRESHOLD.saturating_sub(buf.len());
                select! {
					_ = interval.tick() => Self::flush(&mut buf, &config).await,
					len = receiver.recv_many(&mut buf, max) => {
						// If the queue is closed, stop
						if len == 0 {
							break;
						}
						// If the buffer is full, flush
						if buf.len() >= FLUSH_THRESHOLD {
							Self::flush(&mut buf, &config).await;
						}
					},
					_ = tokio::signal::ctrl_c() => break,
				}
            }
            drop(receiver);
            // Attempt to flush remaining accesses in buffer before stopping
            while !buf.is_empty() {
                Self::flush(&mut buf, &config).await;
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
    async fn flush(pool: &mut Vec<Access>, config: &AccessPoolConfig) {
        if pool.is_empty() {
            return;
        }
        info!(count = pool.len(), "attempt to flush accesses");
        // HTTP request to push accesses
        let client = reqwest::Client::new();
        let res = client
            .put(&config.url)
            .basic_auth(&config.property, Some(&config.secret))
            .json(pool)
            .send()
            .await;
        let response = match res {
            Ok(response) => response,
            Err(error) => {
                error!(config.url, %error, "access: HTTP call failure");
                return;
            }
        };
        let status = response.status();
        if !status.is_success() {
            error!(config.url, %status, "access: HTTP call failure");
            return;
        }
        // Success: clear pool
        info!("clear accesses pool");
        pool.clear();
    }
}

static ACCESS_POOL: LazyLock<AccessPool> = LazyLock::new(|| AccessPool::new());

/// Analytics collection layer.
#[derive(Clone)]
pub struct AnalyticsLayer;

impl<S> Layer<S> for AnalyticsLayer {
    type Service = AnalyticsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AnalyticsMiddleware { inner }
    }
}

/// Middleware collecting analytics data.
#[derive(Clone)]
pub struct AnalyticsMiddleware<S> {
    inner: S,
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
        // Get peer address
        let (mut parts, body) = request.into_parts();
        // According to the crate's documentation, `InsecureClientIp` is fine for geolocation
        let peer_addr = block_on(InsecureClientIp::from_request_parts(&mut parts, &()))
            .expect("could not retrieve peer address")
            .0;
        let request = Request::from_parts(parts, body);
        // Gather data
        let user_agent = request
            .headers()
            .get(USER_AGENT)
            .and_then(|ua| ua.to_str().ok())
            .map(str::to_owned);
        let referer = request
            .headers()
            .get(REFERER)
            .and_then(|ua| ua.to_str().ok())
            .map(str::to_owned);
        let method = request.method().to_string();
        let uri = request.uri().to_string();
        let future = self.inner.call(request);
        Box::pin(async move {
            ACCESS_POOL
                .push(Access {
                    date: Utc::now().timestamp_millis() / 1000,
                    peer_addr: Some(peer_addr),
                    user_agent,
                    referer,
                    method,
                    uri,
                })
                .await;
            let response: Response = future.await?;
            Ok(response)
        })
    }
}
