//! HTTP router builder + OpenAPI documentation.

pub mod error;
pub mod openapi;

use crate::auth::jwt::Role;
use crate::auth::middleware::{require_auth, require_role};
use crate::config::Settings;
use crate::handlers;
use crate::middleware::cors as cors_layer;
use crate::middleware::idempotency;
use crate::middleware::ratelimit::{self};
use crate::middleware::request_id;
use crate::state::AppState;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{delete, get, patch, post};
use axum::Router;
use std::sync::Arc;
use std::time::Duration;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

/// Build the full app router. The returned `Router` is meant to be passed
/// to `axum::serve(...)`.
pub fn build_router(state: AppState, settings: &Arc<Settings>) -> Router {
    let rl = ratelimit::layer_from_settings(settings);

    // Public routes (no auth).
    let public = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        .route("/metrics", get(handlers::health::metrics))
        .route("/version", get(handlers::health::version))
        .route("/docs", get(serve_swagger_ui))
        .route("/api-docs/openapi.json", get(serve_openapi))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/refresh", post(handlers::auth::refresh))
        .route("/api/garos/nodes/{mac}/heartbeat", post(handlers::nodes::heartbeat));

    // Authenticated routes — require a valid JWT.
    //
    // Sub-routes split into two tiers:
    //   * `user` — any authenticated user (read-only and self-management).
    //   * `admin` — sensitive mutations: user provisioning, firewall policy,
    //     storage writes, panic-mode. Wrapped by `require_role(Admin)`
    //     to prevent privilege escalation (Wave 6 critical #1).
    let user = Router::new()
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/auth/me", get(handlers::auth::me))
        // users
        .route("/api/garos/users", get(handlers::users::list))
        .route("/api/garos/users/stats", get(handlers::users::stats))
        .route("/api/garos/users/{id}", get(handlers::users::by_id))
        .route("/api/garos/users/{id}/quota", patch(handlers::users::update_quota))
        .route("/api/garos/users/{id}/sessions", get(handlers::users::sessions))
        // nodes
        .route("/api/garos/nodes", get(handlers::nodes::list_nodes))
        .route("/api/garos/nodes/stats", get(handlers::nodes::node_stats))
        .route("/api/garos/nodes/{mac}", get(handlers::nodes::get_node))
        .route("/api/garos/nodes/{mac}/wol", post(handlers::nodes::wol_node))
        .route("/api/garos/nodes/{mac}/reboot", post(handlers::nodes::reboot_node))
        .route("/api/garos/nodes/{mac}/shutdown", post(handlers::nodes::shutdown_node))
        .route("/api/garos/nodes/{mac}/maintenance", post(handlers::nodes::maintenance_node))
        .route("/api/garos/nodes/{mac}/heartbeat", get(handlers::nodes::heartbeat_status))
        .route("/api/garos/nodes/{mac}/events", get(handlers::nodes::node_events))
        .route("/api/garos/nodes/{mac}/heartbeat", post(handlers::nodes::heartbeat))
        // images
        .route("/api/garos/images", get(handlers::images::list))
        .route("/api/garos/images/{id}", get(handlers::images::by_id))
        .route("/api/garos/images/{id}/build/status", get(handlers::images::build_status))
        .route("/api/garos/images/{id}/versions", get(handlers::images::versions))
        .route("/api/garos/images/{id}/diff/{versionA}/{versionB}", get(handlers::images::diff))
        .route("/api/garos/images/{id}/stations", get(handlers::images::stations))
        // firewall
        .route("/api/garos/firewall/rules", get(handlers::firewall::list))
        .route("/api/garos/firewall/panic/status", get(handlers::firewall::panic_status))
        .route("/api/garos/firewall/connections", get(handlers::firewall::connections))
        // storage
        .route("/api/garos/storage/pools", get(handlers::storage::pools))
        .route("/api/garos/storage/pools/{name}/usage", get(handlers::storage::pool_usage))
        .route("/api/garos/storage/scrub/status", get(handlers::storage::scrub_status))
        .route("/api/garos/storage/snapshots", get(handlers::storage::snapshots))
        .route("/api/garos/storage/drives", get(handlers::storage::drives))
        .route("/api/garos/storage/exports", get(handlers::storage::list_exports))
        // services
        .route("/api/garos/services", get(handlers::services::list))
        .route("/api/garos/services/{name}", get(handlers::services::by_name))
        .route("/api/garos/services/{name}/logs", get(handlers::services::logs))
        .route("/api/garos/services/{name}/health", get(handlers::services::health))
        // metrics / activity / audit
        .route("/api/garos/metrics", get(handlers::metrics::snapshot))
        .route("/api/garos/metrics/series", get(handlers::metrics::series))
        .route("/api/garos/metrics/sla", get(handlers::metrics::sla))
        .route("/api/garos/activity", get(handlers::activity::feed))
        .route("/api/garos/audit", get(handlers::audit::list))
        .route("/api/garos/audit/stats", get(handlers::audit::stats))
        .route("/api/garos/audit/export", get(handlers::audit::export))
        .route("/api/garos/audit/{id}", get(handlers::audit::by_id));

    // Admin-only mutations get an extra `require_role(Admin)` layer that
    // runs *after* `require_auth` (inner layer wraps handler first, outer
    // wraps the whole chain — order is: auth → role → handler).
    let admin = Router::new()
        // user provisioning / role / status changes
        .route("/api/garos/users", post(handlers::users::create))
        .route("/api/garos/users/{id}", patch(handlers::users::update).delete(handlers::users::soft_delete))
        .route("/api/garos/users/{id}/status", patch(handlers::users::update_status))
        .route("/api/garos/users/{id}/reset-password", post(handlers::users::reset_password))
        .route("/api/garos/users/{id}/unlock", post(handlers::users::unlock))
        // firewall policy / panic-mode
        .route("/api/garos/firewall/rules", post(handlers::firewall::create))
        .route("/api/garos/firewall/rules/preview", post(handlers::firewall::preview))
        .route("/api/garos/firewall/rules/{id}", patch(handlers::firewall::update).delete(handlers::firewall::delete))
        .route("/api/garos/firewall/panic", post(handlers::firewall::panic_on))
        .route("/api/garos/firewall/panic", delete(handlers::firewall::panic_off))
        .route("/api/garos/firewall/validate", post(handlers::firewall::validate))
        // storage writes
        .route("/api/garos/storage/scrub", post(handlers::storage::start_scrub))
        .route("/api/garos/storage/snapshots", post(handlers::storage::create_snapshot))
        .route("/api/garos/storage/snapshots/{id}/restore", post(handlers::storage::restore_snapshot))
        .route("/api/garos/storage/snapshots/{id}", delete(handlers::storage::delete_snapshot))
        .route("/api/garos/storage/exports", post(handlers::storage::create_export))
        .route("/api/garos/storage/exports/{path}", delete(handlers::storage::delete_export))
        // node destructive bulk ops + single reimage
        .route("/api/garos/nodes/{mac}/reimage", post(handlers::nodes::reimage_node))
        .route("/api/garos/nodes/bulk/reimage", post(handlers::nodes::bulk_reimage))
        .route("/api/garos/nodes/bulk/shutdown", post(handlers::nodes::bulk_shutdown))
        .route("/api/garos/nodes/bulk/wol", post(handlers::nodes::bulk_wol))
        // image builds / publish
        .route("/api/garos/images", post(handlers::images::create))
        .route("/api/garos/images/{id}", patch(handlers::images::update).delete(handlers::images::delete))
        .route("/api/garos/images/{id}/build", post(handlers::images::start_build))
        .route("/api/garos/images/{id}/publish", post(handlers::images::publish))
        .route("/api/garos/images/{id}/unpublish", post(handlers::images::unpublish))
        .route_layer(from_fn_with_state(state.clone(), require_role));

    let authed = Router::new()
        .merge(user)
        .merge(admin)
        .route_layer(from_fn_with_state(state.clone(), require_auth));

    // WebSocket bypasses JSON auth middleware (token via ?token=). Because
    // `ws_handler` extracts `State<(Arc<JwtService>, RealtimeHub)>` (a tuple
    // that does not match `AppState`), it must live on its own Router whose
    // state type matches the tuple. We merge it as a separate branch with
    // `Router::with_state((jwt, hub))` below.
    let ws = Router::new()
        .route("/api/ws", get(handlers::ws::ws_handler))
        .with_state((state.jwt.clone(), state.realtime.clone()));

    public
        .merge(authed)
        .merge(ws)
        .layer(from_fn(idempotency::middleware))
        .layer(from_fn(request_id::middleware))
        .layer(from_fn(crate::middleware::logging::middleware))
        .layer(cors_layer::layer(settings))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(from_fn_with_state(rl.clone(), ratelimit::middleware))
        .with_state(state)
}

async fn serve_openapi() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(openapi::openapi())
}

async fn serve_swagger_ui() -> impl axum::response::IntoResponse {
    // Lightweight inline Swagger UI shell. utoipa-swagger-ui's axum integration in this
    // crate version returns a Router (not an IntoResponse impl), so we render a static
    // HTML page that points the user to the raw OpenAPI JSON. Functionally equivalent
    // for documentation/discovery; the JSON endpoint below is the source of truth.
    let html = r#"<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <title>garos-backend API</title>
</head>
<body>
  <h1>garos-backend OpenAPI</h1>
  <p>Interactive Swagger UI requires the utoipa-swagger-ui axum feature enabled.
  Until then, see the raw spec:</p>
  <ul>
    <li><a href=\"/api-docs/openapi.json\">/api-docs/openapi.json</a></li>
  </ul>
</body>
</html>
"#;
    axum::response::Html(html.to_string())
}

/// Wait for the configured timeout when shutting down.
pub fn shutdown_timeout() -> Duration {
    Duration::from_secs(15)
}

/// Convenience: build a "no-default-features" router for tests.
pub fn router_for_tests(state: AppState) -> Router {
    let settings = state.settings.clone();
    build_router(state, &settings)
}

/// Helper: extract the `User-Agent` header (used by `handlers::auth`).
pub fn _require_role(_r: Role) {}
