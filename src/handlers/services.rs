//! systemd service handlers.

use crate::auth::extractor::CurrentUser;
use crate::domain::service::*;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LogsQuery {
    pub lines: Option<u32>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub priority: Option<String>,
}

/// `GET /api/garos/services`
pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<ServiceView>>, AppError> {
    let s = state.services.list().await?;
    Ok(Json(s))
}

/// `GET /api/garos/services/{name}`
pub async fn by_name(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(name): Path<String>,
) -> Result<Json<ServiceView>, AppError> {
    let s = state.services.by_name(&name).await?;
    Ok(Json(s))
}

/// `POST /api/garos/services/{name}/start`
pub async fn start(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(name): Path<String>,
) -> Result<(), AppError> {
    state.services.start(&name).await
}

/// `POST /api/garos/services/{name}/stop`
pub async fn stop(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(name): Path<String>,
) -> Result<(), AppError> {
    state.services.stop(&name).await
}

/// `POST /api/garos/services/{name}/restart`
pub async fn restart(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(name): Path<String>,
) -> Result<(), AppError> {
    state.services.restart(&name).await
}

/// `GET /api/garos/services/{name}/logs`
pub async fn logs(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(name): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<Vec<LogLine>>, AppError> {
    let lines = q.lines.unwrap_or(100).clamp(1, 10_000);
    let l = state
        .services
        .logs(&name, lines, q.since.as_deref(), q.until.as_deref(), q.priority.as_deref())
        .await?;
    Ok(Json(l))
}

/// `GET /api/garos/services/{name}/health`
pub async fn health(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(name): Path<String>,
) -> Result<Json<ServiceHealth>, AppError> {
    let h = state.services.health(&name).await?;
    Ok(Json(h))
}
