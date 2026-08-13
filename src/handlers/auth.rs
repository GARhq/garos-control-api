//! Auth handlers: login, refresh, logout, me.

use crate::auth::extractor::CurrentUser;
use crate::domain::user::*;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MeResponse {
    pub id: uuid::Uuid,
    pub username: String,
    pub role: String,
    pub status: String,
}

/// `POST /api/auth/login`
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = crate::error::ErrorBody),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    req.validate()?;
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let resp = state
        .users
        .login(req, Some(&addr.ip().to_string()), user_agent.as_deref())
        .await?;
    Ok(Json(resp))
}

/// `POST /api/auth/refresh`
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "New tokens", body = LoginResponse),
        (status = 401, description = "Invalid refresh token", body = crate::error::ErrorBody),
    ),
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let resp = state.users.refresh(&req.refresh_token).await?;
    Ok(Json(resp))
}

/// `POST /api/auth/logout`
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 204, description = "Logged out"),
    ),
)]
pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.users.logout(&req.refresh_token).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// `GET /api/auth/me`
#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current user", body = MeResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorBody),
    ),
    security(("bearer" = []))
)]
pub async fn me(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<MeResponse>, AppError> {
    let row = state
        .users
        .by_id(&user.id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    Ok(Json(MeResponse {
        id: row.id(),
        username: row.username,
        role: row.role,
        status: row.status,
    }))
}
