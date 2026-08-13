//! CORS layer builder.

use crate::config::Settings;
use axum::http::{header, HeaderValue, Method};
use tower_http::cors::CorsLayer;

pub fn layer(settings: &Settings) -> CorsLayer {
    let allowed = if settings.cors.allowed_origins.is_empty() {
        vec!["*".to_string()]
    } else {
        settings.cors.allowed_origins.clone()
    };

    let origins: Vec<HeaderValue> = allowed
        .iter()
        .filter_map(|s| HeaderValue::from_str(s).ok())
        .collect();

    let methods = [
        Method::GET,
        Method::POST,
        Method::PATCH,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::HEAD,
    ];

    let headers = [
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::HeaderName::from_static("x-request-id"),
        header::HeaderName::from_static("idempotency-key"),
    ];

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(methods)
        .allow_headers(headers)
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(60 * 60))
}
