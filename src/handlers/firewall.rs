//! Firewall handlers.

use crate::auth::extractor::CurrentUser;
use crate::domain::firewall::*;
use crate::error::AppError;
use crate::state::AppState;
use validator::Validate;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ConnectionsQuery {
    pub limit: Option<usize>,
    pub protocol: Option<String>,
    pub state: Option<String>,
}

/// `GET /api/garos/firewall/rules`
pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<FirewallRuleView>>, AppError> {
    let rules = state.firewall.list_view().await?;
    Ok(Json(rules))
}

/// `POST /api/garos/firewall/rules`
pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<FirewallRuleCreate>,
) -> Result<Json<FirewallRuleView>, AppError> {
    if user.role < crate::auth::jwt::Role::Operator {
        return Err(AppError::Forbidden);
    }
    let v = state.firewall.create(req, &user.username).await?;
    Ok(Json(v))
}

/// `GET /api/garos/firewall/rules/{id}`
pub async fn by_id(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<FirewallRuleView>, AppError> {
    let v = state.firewall.by_id_view(&id).await?;
    Ok(Json(v))
}

/// `PATCH /api/garos/firewall/rules/{id}`
pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<FirewallRuleUpdate>,
) -> Result<Json<FirewallRuleView>, AppError> {
    let v = state.firewall.update(&id, req, &user.username).await?;
    Ok(Json(v))
}

/// `DELETE /api/garos/firewall/rules/{id}`
pub async fn delete(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    state.firewall.delete(&id, &user.username).await
}

/// `POST /api/garos/firewall/rules/preview`
pub async fn preview(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<FirewallRuleCreate>,
) -> Result<Json<FirewallRulePreview>, AppError> {
    let p = state.firewall.preview(req).await?;
    Ok(Json(p))
}

/// `POST /api/garos/firewall/panic`
pub async fn panic_on(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<PanicStatus>, AppError> {
    let s = state.firewall.panic(true, &user.username).await?;
    Ok(Json(s))
}

/// `DELETE /api/garos/firewall/panic`
pub async fn panic_off(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<PanicStatus>, AppError> {
    let s = state.firewall.panic(false, &user.username).await?;
    Ok(Json(s))
}

/// `GET /api/garos/firewall/panic/status`
pub async fn panic_status(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<PanicStatus>, AppError> {
    let s = state.firewall.panic_status().await?;
    Ok(Json(s))
}

/// `GET /api/garos/firewall/connections`
pub async fn connections(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(q): Query<ConnectionsQuery>,
) -> Result<Json<Vec<ConnectionEntry>>, AppError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 1000);
    let conns = state
        .firewall
        .connections(limit, q.protocol.as_deref(), q.state.as_deref())
        .await?;
    Ok(Json(conns))
}

/// `POST /api/garos/firewall/validate`
pub async fn validate(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<String>>, AppError> {
    let v = state.firewall.validate().await?;
    Ok(Json(v))
}
