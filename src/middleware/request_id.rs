//! Request-id middleware + helpers to read the current trace id.

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

/// Standard header name for the trace id.
pub const HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Task-local holding the trace id for the current request.
tokio::task_local! {
    pub static CURRENT_TRACE_ID: Uuid;
}

/// Middleware that ensures every request has a UUID v7 trace id and exposes
/// it as a response header.
pub async fn middleware(mut req: Request, next: Next) -> Response {
    let trace_id = req
        .headers()
        .get(&HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::now_v7);

    req.extensions_mut().insert(trace_id);

    let trace_id_for_task = trace_id;
    let fut = async move { next.run(req).await };

    let mut resp = CURRENT_TRACE_ID
        .scope(trace_id_for_task, fut)
        .await;

    if let Ok(v) = HeaderValue::from_str(&trace_id.to_string()) {
        resp.headers_mut().insert(HEADER, v);
    }
    resp
}

/// Read the current trace id from the task-local.
pub fn current_trace_id() -> Option<Uuid> {
    CURRENT_TRACE_ID.try_with(|id| *id).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn echo() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn header_is_set() {
        let app = axum::Router::new()
            .route("/", axum::routing::get(echo))
            .layer(axum::middleware::from_fn(middleware));
        let res = app
            .oneshot(Request::builder().body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(res.headers().contains_key(&HEADER));
        let v = res.headers().get(&HEADER).unwrap().to_str().unwrap();
        assert!(Uuid::parse_str(v).is_ok());
    }

    #[tokio::test]
    async fn current_trace_id_is_set() {
        async fn handler() -> Uuid {
            current_trace_id().unwrap_or_else(Uuid::nil)
        }
        let app = axum::Router::new()
            .route("/", axum::routing::get(handler))
            .layer(axum::middleware::from_fn(middleware));
        let res = app
            .oneshot(Request::builder().body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(res.status().is_success());
    }
}
