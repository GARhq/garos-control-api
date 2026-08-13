//! Storage handlers.

use crate::auth::extractor::CurrentUser;
use crate::domain::storage::*;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct PoolNamePath {
    pub name: String,
}

/// `GET /api/garos/storage/pools`
pub async fn pools(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<StoragePool>>, AppError> {
    let p = state.storage.pools().await?;
    Ok(Json(p))
}

/// `GET /api/garos/storage/pools/{name}/usage`
pub async fn pool_usage(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(name): Path<String>,
) -> Result<Json<StoragePool>, AppError> {
    let p = state.storage.usage(&name).await?;
    Ok(Json(p))
}

/// `POST /api/garos/storage/scrub`
pub async fn start_scrub(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ScrubStatus>, AppError> {
    let pool = body
        .get("pool")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("pool required".into()))?;
    let s = state.storage.start_scrub(pool).await?;
    Ok(Json(s))
}

/// `GET /api/garos/storage/scrub/status`
pub async fn scrub_status(
    State(state): State<AppState>,
    _user: CurrentUser,
    axum::extract::Query(q): axum::extract::Query<serde_json::Value>,
) -> Result<Json<ScrubStatus>, AppError> {
    let pool = q
        .get("pool")
        .and_then(|v| v.as_str())
        .unwrap_or("garos");
    let s = state.storage.scrub_status(pool).await?;
    Ok(Json(s))
}

/// `GET /api/garos/storage/snapshots`
pub async fn snapshots(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<Snapshot>>, AppError> {
    let s = state.storage.snapshots().await?;
    Ok(Json(s))
}

/// `POST /api/garos/storage/snapshots`
pub async fn create_snapshot(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<SnapshotCreate>,
) -> Result<Json<Snapshot>, AppError> {
    let s = state.storage.create_snapshot(req, &user.username).await?;
    Ok(Json(s))
}

/// `POST /api/garos/storage/snapshots/{id}/restore`
pub async fn restore_snapshot(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<(), AppError> {
    let target = body
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("target required".into()))?;
    state.storage.restore_snapshot(&id, target, &user.username).await
}

/// `DELETE /api/garos/storage/snapshots/{id}`
pub async fn delete_snapshot(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    state.storage.delete_snapshot(&id, &user.username).await
}

/// `GET /api/garos/storage/drives`
pub async fn drives(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<Drive>>, AppError> {
    let d = state.storage.drives().await?;
    Ok(Json(d))
}

/// `GET /api/garos/storage/exports`
pub async fn list_exports(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<NfsExport>>, AppError> {
    let e = state.storage.exports().await?;
    Ok(Json(e))
}

/// `POST /api/garos/storage/exports`
pub async fn create_export(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<NfsExportSpec>,
) -> Result<Json<NfsExport>, AppError> {
    let e = state.storage.create_export(req, &user.username).await?;
    Ok(Json(e))
}

/// `DELETE /api/garos/storage/exports/{path}`
pub async fn delete_export(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(path): Path<String>,
) -> Result<(), AppError> {
    state.storage.delete_export(&path, &user.username).await
}
