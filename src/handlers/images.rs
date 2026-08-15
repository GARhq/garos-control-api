//! Image handlers.

use crate::auth::extractor::CurrentUser;
use crate::db::models::image::ImageRow;
use crate::domain::image::*;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

fn image_view(row: ImageRow) -> ImageView {
    ImageView {
        id: row.id_uuid(),
        name: row.name,
        description: row.description,
        nixos_version: row.nixos_version,
        kernel: row.kernel,
        kernel_args: row.kernel_args,
        size_mb: row.size_mb,
        status: row.status,
        version: row.version,
        parent_id: row.parent_id.and_then(|s| Uuid::parse_str(&s).ok()),
        published_at: row.published_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

/// `GET /api/garos/images`
pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<ImageView>>, AppError> {
    let rows = state.images.list().await?;
    Ok(Json(rows.into_iter().map(image_view).collect()))
}

/// `POST /api/garos/images`
pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<ImageCreate>,
) -> Result<Json<ImageView>, AppError> {
    let row = state.images.create(req, &user.id).await?;
    Ok(Json(image_view(row)))
}

/// `GET /api/garos/images/{id}`
pub async fn by_id(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ImageView>, AppError> {
    let row = state
        .images
        .by_id(&id)
        .await?
        .ok_or(AppError::NotFound("image".into()))?;
    Ok(Json(image_view(row)))
}

/// `PATCH /api/garos/images/{id}`
pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ImageUpdate>,
) -> Result<Json<ImageView>, AppError> {
    let row = state.images.update(&id, req, &user.username).await?;
    Ok(Json(image_view(row)))
}

/// `DELETE /api/garos/images/{id}`
pub async fn delete(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    state.images.delete(&id, &user.username).await
}

/// `POST /api/garos/images/{id}/build`
pub async fn start_build(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ImageBuildStatus>, AppError> {
    let s = state.images.start_build(&id).await?;
    Ok(Json(s))
}

/// `GET /api/garos/images/{id}/build/status`
pub async fn build_status(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ImageBuildStatus>, AppError> {
    let s = state.images.build_status(&id).await?;
    Ok(Json(s))
}

/// `POST /api/garos/images/{id}/publish`
pub async fn publish(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    state.images.publish(&id, &user.username).await
}

/// `POST /api/garos/images/{id}/unpublish`
pub async fn unpublish(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    state.images.unpublish(&id, &user.username).await
}

/// `GET /api/garos/images/{id}/versions`
pub async fn versions(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = state.images.list_versions(&id).await?;
    let count = rows.len();
    Ok(Json(serde_json::json!({ "count": count, "items": rows })))
}

/// `GET /api/garos/images/{id}/diff/{versionA}/{versionB}`
pub async fn diff(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path((id, a, b)): Path<(Uuid, String, String)>,
) -> Result<Json<ImageDiff>, AppError> {
    let d = state.images.diff(&id, &a, &b).await?;
    Ok(Json(d))
}

/// `GET /api/garos/images/{id}/stations`
pub async fn stations(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let s = state.images.stations(&id).await?;
    Ok(Json(s))
}
