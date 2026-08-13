//! Health, version, and metrics handlers.

use crate::config::Settings;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VersionInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub git_sha: &'static str,
    pub build_time: &'static str,
    pub rustc_version: &'static str,
    pub target: &'static str,
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `GET /health`
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses((status = 200, description = "Liveness probe", body = serde_json::Value)),
)]
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": NAME,
    }))
}

/// `GET /ready`
#[utoipa::path(
    get,
    path = "/ready",
    tag = "system",
    responses(
        (status = 200, description = "Ready"),
        (status = 503, description = "Not ready", body = crate::error::ErrorBody),
    ),
)]
pub async fn ready(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    // Check DB
    let row: (i64,) = sqlx::query_as("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("db: {e}")))?;
    // Skip integration probes if mocks enabled
    let mut checks = serde_json::Map::new();
    checks.insert("db".into(), serde_json::json!(row.0 == 1));
    Ok(Json(serde_json::json!({ "ready": true, "checks": checks })))
}

/// `GET /metrics` (Prometheus text)
pub async fn metrics(State(state): State<AppState>) -> Result<Response, AppError> {
    let body = state.metrics.render().map_err(|e| {
        AppError::Internal(anyhow::anyhow!("metrics render: {e}"))
    })?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
        .into_response())
}

/// `GET /version`
pub async fn version() -> Json<VersionInfo> {
    Json(VersionInfo {
        name: NAME,
        version: VERSION,
        git_sha: option_env!("GIT_SHA").unwrap_or("unknown"),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown"),
        rustc_version: rustc_version_runtime(),
        target: std::env::consts::ARCH,
    })
}

fn rustc_version_runtime() -> &'static str {
    // Compiled-in at build time via env in Cargo.toml build script.
    // We expose a static slice for the lifetime of the program.
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        std::env::vars()
            .find(|(k, _)| k == "RUSTC_VERSION")
            .map(|(_, v)| v)
            .unwrap_or_else(|| "unknown".into())
    })
    .leak()
}

#[allow(dead_code)]
pub fn build_version(_settings: &Arc<Settings>) -> VersionInfo {
    VersionInfo {
        name: NAME,
        version: VERSION,
        git_sha: option_env!("GIT_SHA").unwrap_or("unknown"),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown"),
        rustc_version: rustc_version_runtime(),
        target: std::env::consts::ARCH,
    }
}
