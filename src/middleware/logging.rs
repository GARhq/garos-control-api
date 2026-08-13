//! HTTP request/response logging middleware.

use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;
use tracing::Span;
use tracing::field::Empty;

/// Trace every request and record status + duration.
pub async fn middleware(req: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let span = tracing::info_span!(
        "http",
        method = %method,
        uri = %uri,
        status = Empty,
        duration_ms = Empty,
    );
    let _enter = span.enter();

    let resp = next.run(req).await;
    let status = resp.status();
    let elapsed_ms = started.elapsed().as_millis() as u64;

    Span::current().record("status", status.as_u16());
    Span::current().record("duration_ms", elapsed_ms);

    if status.is_server_error() {
        tracing::error!(target: "http", "request failed");
    } else if status.is_client_error() {
        tracing::info!(target: "http", "request rejected");
    } else {
        tracing::info!(target: "http", "request complete");
    }

    // Increment Prometheus counter via a side-channel; keep this middleware
    // dependency-free. The actual counter is on AppState.
    drop(_enter);
    drop(span);
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    async fn ok() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn passes_through() {
        let app = Router::new()
            .route("/", get(ok))
            .layer(axum::middleware::from_fn(middleware));
        let r = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(r.status().is_success());
    }
}
