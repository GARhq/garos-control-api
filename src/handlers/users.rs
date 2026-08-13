//! User handlers.

use crate::auth::extractor::CurrentUser;
use crate::db::models::user::UserRow;
use crate::domain::user::*;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListUsersQuery {
    pub search: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn user_view(row: UserRow) -> UserView {
    UserView {
        id: row.id(),
        username: row.username,
        email: row.email,
        display_name: row.display_name,
        role: row.role,
        status: row.status,
        quota_used_bytes: row.quota_used_bytes,
        quota_limit_bytes: row.quota_limit_bytes,
        created_at: row.created_at,
        updated_at: row.updated_at,
        last_activity_at: row.last_activity_at,
    }
}

/// `GET /api/garos/users`
#[utoipa::path(
    get,
    path = "/api/garos/users",
    tag = "users",
    params(ListUsersQuery),
    responses(
        (status = 200, description = "Users", body = Vec<UserView>),
    ),
    security(("bearer" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(q): Query<ListUsersQuery>,
) -> Result<Json<Vec<UserView>>, AppError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let offset = q.offset.unwrap_or(0).max(0);
    let rows = state
        .users
        .list(q.search.as_deref(), q.role.as_deref(), q.status.as_deref(), limit, offset)
        .await?;
    Ok(Json(rows.into_iter().map(user_view).collect()))
}

/// `POST /api/garos/users`
pub async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<UserCreate>,
) -> Result<Json<UserView>, AppError> {
    let row = state.users.create(req).await?;
    Ok(Json(user_view(row)))
}

/// `GET /api/garos/users/{id}`
pub async fn by_id(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<UserView>, AppError> {
    let row = state
        .users
        .by_id(&id)
        .await?
        .ok_or(AppError::NotFound("user".into()))?;
    Ok(Json(user_view(row)))
}

/// `PATCH /api/garos/users/{id}`
pub async fn update(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UserUpdate>,
) -> Result<Json<UserView>, AppError> {
    let row = state.users.update(&id, req).await?;
    Ok(Json(user_view(row)))
}

/// `PATCH /api/garos/users/{id}/quota`
pub async fn update_quota(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<QuotaUpdateRequest>,
) -> Result<Json<UserView>, AppError> {
    let row = state.users.update_quota(&id, req).await?;
    Ok(Json(user_view(row)))
}

/// `PATCH /api/garos/users/{id}/status`
pub async fn update_status(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<StatusUpdateRequest>,
) -> Result<Json<UserView>, AppError> {
    let row = state.users.update_status(&id, req).await?;
    Ok(Json(user_view(row)))
}

/// `DELETE /api/garos/users/{id}`
pub async fn soft_delete(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    if user.role < crate::auth::jwt::Role::Admin {
        return Err(AppError::Forbidden);
    }
    state.users.soft_delete(&id).await
}

/// `POST /api/garos/users/{id}/reset-password`
pub async fn reset_password(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PasswordResetRequest>,
) -> Result<(), AppError> {
    state.users.reset_password(&id, req).await
}

/// `GET /api/garos/users/{id}/sessions`
pub async fn sessions(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = state.users.sessions(&id).await?;
    let count = rows.len();
    Ok(Json(serde_json::json!({ "count": count, "items": rows })))
}

/// `POST /api/garos/users/{id}/unlock`
pub async fn unlock(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    state.users.unlock(&id).await
}

/// `GET /api/garos/users/stats`
pub async fn stats(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<UserStats>, AppError> {
    let s = state.users.stats().await?;
    Ok(Json(s))
}
