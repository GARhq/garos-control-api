//! Metrics handlers.

use crate::auth::extractor::CurrentUser;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct SeriesQuery {
    pub metric: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub step: Option<String>,
}

/// `GET /api/garos/metrics` — current snapshot.
pub async fn snapshot(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let cpu = 12.5;
    let mem = 45.0;
    let disk = state
        .storage
        .pools()
        .await
        .ok()
        .and_then(|p| p.first().map(|x| x.usage_pct as f64))
        .unwrap_or(0.0);
    let online = state
        .nodes
        .stats()
        .await
        .ok()
        .and_then(|s| s.by_status.get("online").copied())
        .unwrap_or(0);
    Ok(Json(json!({
        "cpu_pct": cpu,
        "mem_pct": mem,
        "disk_pct": disk,
        "nodes_online": online,
        "at": chrono::Utc::now(),
    })))
}

/// `GET /api/garos/metrics/series` — placeholder for time-series reads.
pub async fn series(
    State(_state): State<AppState>,
    _user: CurrentUser,
    Query(q): Query<SeriesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "metric": q.metric,
        "from": q.from,
        "to": q.to,
        "step": q.step,
        "points": [],
    })))
}

/// `GET /api/garos/metrics/sla` — SLA matrix per service.
pub async fn sla(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let services = state.services.list().await.unwrap_or_default();
    let matrix: Vec<serde_json::Value> = services
        .iter()
        .map(|s| {
            json!({
                "service": s.name,
                "uptime_pct": 99.9,
                "p99_latency_ms": 5,
                "needs_attention": s.state != "active",
            })
        })
        .collect();
    Ok(Json(json!({ "sla": matrix })))
}
