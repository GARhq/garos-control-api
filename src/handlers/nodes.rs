//! Node handlers.

use crate::auth::extractor::CurrentUser;
use crate::domain::node::*;
use crate::error::AppError;
use crate::state::AppState;
use validator::Validate;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListQuery {
    pub status: Option<String>,
    pub image_id: Option<String>,
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default = "default_order")]
    pub order: String,
}

fn default_limit() -> i64 {
    50
}
fn default_sort() -> String {
    "mac".into()
}
fn default_order() -> String {
    "asc".into()
}

/// `GET /api/garos/nodes`
#[utoipa::path(
    get,
    path = "/api/garos/nodes",
    tag = "nodes",
    params(ListQuery),
    responses(
        (status = 200, description = "List of nodes", body = Vec<NetbootDevice>),
    ),
    security(("bearer" = []))
)]
pub async fn list_nodes(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<NetbootDevice>>, AppError> {
    let limit = q.limit.clamp(1, 500);
    let offset = q.offset.max(0);
    let rows = state
        .nodes
        .list(
            q.status.as_deref(),
            q.image_id.as_deref(),
            q.search.as_deref(),
            limit,
            offset,
            &q.sort,
            &q.order,
        )
        .await?;
    Ok(Json(rows.into_iter().map(node_to_view).collect()))
}

/// `GET /api/garos/nodes/{mac}`
#[utoipa::path(
    get,
    path = "/api/garos/nodes/{mac}",
    tag = "nodes",
    params(("mac" = String, Path,)),
    responses(
        (status = 200, description = "Node", body = NetbootDevice),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    security(("bearer" = []))
)]
pub async fn get_node(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(mac): Path<String>,
) -> Result<Json<NetbootDevice>, AppError> {
    let row = state
        .nodes
        .by_mac(&mac)
        .await?
        .ok_or(AppError::NotFound(format!("node {mac}")))?;
    Ok(Json(node_to_view(row)))
}

/// `POST /api/garos/nodes/{mac}/wol`
#[utoipa::path(
    post,
    path = "/api/garos/nodes/{mac}/wol",
    tag = "nodes",
    params(("mac" = String, Path,)),
    responses(
        (status = 202, description = "Magic packet sent", body = WolResult),
        (status = 400, description = "Invalid MAC", body = crate::error::ErrorBody),
    ),
    security(("bearer" = []))
)]
pub async fn wol_node(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(mac): Path<String>,
) -> Result<Json<WolResult>, AppError> {
    let r = state.nodes.wol(&mac).await?;
    Ok(Json(r))
}

/// `POST /api/garos/nodes/{mac}/reboot`
#[utoipa::path(
    post,
    path = "/api/garos/nodes/{mac}/reboot",
    tag = "nodes",
    params(("mac" = String, Path,)),
    responses(
        (status = 202, description = "Reboot scheduled", body = NetbootDevice),
    ),
    security(("bearer" = []))
)]
pub async fn reboot_node(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(mac): Path<String>,
) -> Result<Json<NetbootDevice>, AppError> {
    let row = state
        .nodes
        .reboot(&mac, &user.username, Some(&user.jti))
        .await?;
    Ok(Json(node_to_view(row)))
}

/// `POST /api/garos/nodes/{mac}/shutdown`
pub async fn shutdown_node(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(mac): Path<String>,
) -> Result<Json<NetbootDevice>, AppError> {
    let row = state
        .nodes
        .shutdown(&mac, &user.username, Some(&user.jti))
        .await?;
    Ok(Json(node_to_view(row)))
}

/// `POST /api/garos/nodes/{mac}/maintenance`
pub async fn maintenance_node(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(mac): Path<String>,
) -> Result<Json<NetbootDevice>, AppError> {
    let row = state.nodes.maintenance(&mac).await?;
    Ok(Json(node_to_view(row)))
}

/// `POST /api/garos/nodes/{mac}/reimage`
pub async fn reimage_node(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(mac): Path<String>,
    Json(req): Json<ReimageRequest>,
) -> Result<Json<NetbootDevice>, AppError> {
    req.validate()?;
    let row = state.nodes.reimage(&mac, &req.image_id, &user.username).await?;
    Ok(Json(node_to_view(row)))
}

/// `POST /api/garos/nodes/{mac}/heartbeat`
pub async fn heartbeat(
    State(state): State<AppState>,
    Path(mac): Path<String>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<NetbootDevice>, AppError> {
    let row = state.nodes.heartbeat(&mac, req).await?;
    Ok(Json(node_to_view(row)))
}

/// `GET /api/garos/nodes/{mac}/heartbeat`
pub async fn heartbeat_status(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(mac): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = state
        .nodes
        .by_mac(&mac)
        .await?
        .ok_or(AppError::NotFound(format!("node {mac}")))?;
    Ok(Json(serde_json::json!({
        "mac": row.mac,
        "status": row.status,
        "last_heartbeat_at": row.last_heartbeat_at,
        "ping_ms": row.ping_ms,
        "cpu_temp_c": row.cpu_temp_c,
    })))
}

/// `GET /api/garos/nodes/{mac}/events`
pub async fn node_events(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(mac): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use crate::domain::audit::AuditQuery;
    let q = AuditQuery {
        actor: None,
        action: None,
        target: Some(mac.clone()),
        from: None,
        to: None,
        limit: Some(50),
        offset: Some(0),
    };
    let rows = state.audit.list(q).await?;
    let count = rows.len();
    Ok(Json(serde_json::json!({ "count": count, "items": rows })))
}

/// `POST /api/garos/nodes/bulk/wol`
#[utoipa::path(
    post,
    path = "/api/garos/nodes/bulk/wol",
    tag = "nodes",
    request_body = BulkMacRequest,
    responses(
        (status = 200, description = "Bulk WOL triggered", body = BulkActionResult),
        (status = 400, description = "Invalid request", body = crate::error::ErrorBody),
    ),
    security(("bearer" = []))
)]
pub async fn bulk_wol(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<BulkMacRequest>,
) -> Result<Json<BulkActionResult>, AppError> {
    let r = state.nodes.bulk_wol(req).await?;
    Ok(Json(r))
}

/// `POST /api/garos/nodes/bulk/shutdown`
#[utoipa::path(
    post,
    path = "/api/garos/nodes/bulk/shutdown",
    tag = "nodes",
    request_body = BulkMacRequest,
    responses(
        (status = 200, description = "Bulk shutdown triggered", body = BulkActionResult),
        (status = 400, description = "Invalid request", body = crate::error::ErrorBody),
    ),
    security(("bearer" = []))
)]
pub async fn bulk_shutdown(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<BulkMacRequest>,
) -> Result<Json<BulkActionResult>, AppError> {
    let r = state.nodes.bulk_shutdown(req, &user.username).await?;
    Ok(Json(r))
}

/// `POST /api/garos/nodes/bulk/reimage`
#[utoipa::path(
    post,
    path = "/api/garos/nodes/bulk/reimage",
    tag = "nodes",
    request_body = BulkReimageRequest,
    responses(
        (status = 200, description = "Bulk reimage triggered", body = BulkActionResult),
        (status = 400, description = "Invalid request", body = crate::error::ErrorBody),
    ),
    security(("bearer" = []))
)]
pub async fn bulk_reimage(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<BulkReimageRequest>,
) -> Result<Json<BulkActionResult>, AppError> {
    let r = state.nodes.bulk_reimage(req, &user.username).await?;
    Ok(Json(r))
}

/// `GET /api/garos/nodes/stats`
#[utoipa::path(
    get,
    path = "/api/garos/nodes/stats",
    tag = "nodes",
    responses(
        (status = 200, description = "Node stats", body = NodeStats),
    ),
    security(("bearer" = []))
)]
pub async fn node_stats(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<NodeStats>, AppError> {
    let s = state.nodes.stats().await?;
    Ok(Json(s))
}

pub fn node_to_view(row: crate::db::models::node::NodeRow) -> NetbootDevice {
    let id = Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil());
    NetbootDevice {
        id,
        mac: row.mac,
        hostname: row.hostname,
        ip: row.ip,
        status: row.status,
        image_id: row.image_id,
        last_heartbeat_at: row.last_heartbeat_at,
        last_seen_at: row.last_seen_at,
        cpu_temp_c: row.cpu_temp_c,
        cpu_usage_pct: row.cpu_usage_pct,
        mem_usage_pct: row.mem_usage_pct,
        fan_rpm: row.fan_rpm,
        ping_ms: row.ping_ms,
        nfs_latency_ms: row.nfs_latency_ms,
        hardware_model: row.hardware_model,
        current_user_id: row.current_user_id,
        current_user_role: row.current_user_role,
        login_at: row.login_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
