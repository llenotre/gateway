//! Logging layer.

use std::task::{Context, Poll};
use axum::extract::Request;
use axum::response::Response;
use chrono::{Utc};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};
use tracing::info;
use crate::util::{date_format, extract_peer_addr};

/// Layer printing logs.
#[derive(Clone)]
pub struct LogLayer;

impl<S> Layer<S> for LogLayer {
    type Service = LogMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LogMiddleware {
            inner,
        }
    }
}

#[derive(Clone)]
pub struct LogMiddleware<S> {
    inner: S,
}

impl<S> Service<Request> for LogMiddleware<S>
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
        let begin = Utc::now();
        let ts = begin.format(date_format::FORMAT);
        let (request, peer_addr) = extract_peer_addr(request);
        let path = request.uri().clone();
        let future = self.inner.call(request);
        Box::pin(async move {
            let response: Response = future.await?;
            let status = response.status().as_u16();
            // Cannot be negative unless going back in time
            let duration = (Utc::now() - begin).num_milliseconds();
            match peer_addr {
                Some(peer_addr) => info!("[{ts}] {peer_addr}: {path} - Response: {status} (in {duration} ms)"),
                None => info!("[{ts}] ???: {path} - Response: {status} (in {duration} ms)"),
            }
            Ok(response)
        })
    }
}
