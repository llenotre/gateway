use std::net::{IpAddr};
use std::task::{Context, Poll};
use std::time::Instant;
use axum::extract::Request;
use axum::http::header::{REFERER, USER_AGENT};
use axum::response::Response;
use chrono::{Duration, Utc};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tower::Service;

const FLUSH_THRESHOLD: usize = 1024;
const FLUSH_INTERVAL: Duration = Duration::minutes(10);

/// TODO doc
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

/// Middleware collecting analytics data.
#[derive(Clone)]
pub struct AnalyticsMiddleware<S> {
    inner: S,

    pool: Vec<Access>,
    last_flush: Instant,
    // TODO flush task
}

impl<S> Service<Request> for AnalyticsMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // TODO get connect info
        let user_agent = request.headers().get(USER_AGENT).and_then(|ua| ua.to_str().ok()).map(str::to_owned);
        let referer = request.headers().get(REFERER).and_then(|ua| ua.to_str().ok()).map(str::to_owned);
        let method = request.method().to_string();
        let uri = request.uri().to_string();
        let access = Access {
            date: Utc::now().timestamp_millis() / 1000,
            peer_addr: None,
            user_agent,
            referer,
            method,
            uri,
        };
        self.pool.push(access);
        // TODO if this is the first access in the pool, launch flush task
        // TODO if reaching the threshold, flush
        let future = self.inner.call(request);
        Box::pin(async move {
            let response: Response = future.await?;
            Ok(response)
        })
    }
}