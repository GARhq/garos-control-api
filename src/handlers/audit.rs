//! Audit log handlers.

use crate::auth::extractor::CurrentUser;
use crate::domain::audit::{AuditEntry, AuditQuery, AuditStats};
use crate::error::AppError;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, Response, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct AuditListQuery {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub target: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn parse_dt(s: Option<String>) -> Result<Option<DateTime<Utc>>, AppError> {
    Ok(match s {
        Some(s) => Some(
            DateTime::parse_from_rfc3339(&s)
                .map_err(|e| AppError::BadRequest(format!("bad date: {e}")))?
                .with_timezone(&Utc),
        ),
        None => None,
    })
}

/// `GET /api/garos/audit`
pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(q): Query<AuditListQuery>,
) -> Result<Json<Vec<AuditEntry>>, AppError> {
    let q = AuditQuery {
        actor: q.actor,
        action: q.action,
        target: q.target,
        from: parse_dt(q.from)?,
        to: parse_dt(q.to)?,
        limit: q.limit,
        offset: q.offset,
    };
    let rows = state.audit.list(q).await?;
    Ok(Json(rows))
}

/// `GET /api/garos/audit/{id}`
pub async fn by_id(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<AuditEntry>, AppError> {
    let e = state.audit.by_id(&id).await?;
    Ok(Json(e))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ExportQuery {
    pub format: Option<String>,
    pub actor: Option<String>,
    pub action: Option<String>,
    pub target: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// `GET /api/garos/audit/export`
pub async fn export(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(q): Query<ExportQuery>,
) -> Result<Response<Body>, AppError> {
    let fmt = q.format.clone().unwrap_or_else(|| "json".into());
    let query = AuditQuery {
        actor: q.actor,
        action: q.action,
        target: q.target,
        from: parse_dt(q.from)?,
        to: parse_dt(q.to)?,
        limit: q.limit,
        offset: q.offset,
    };
    let body = state.audit.export(&fmt, query).await?;
    let content_type = match fmt.as_str() {
        "cef" => "text/plain",
        "leef" => "text/plain",
        _ => "application/json",
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .unwrap())
}

/// `GET /api/garos/audit/stats`
pub async fn stats(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<AuditStats>, AppError> {
    let s = state.audit.stats().await?;
    Ok(Json(s))
}
