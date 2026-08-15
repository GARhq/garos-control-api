//! utoipa OpenAPI definition.

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "garos-backend",
        version = "0.1.0",
        description = "Production HTTP API for the kryonix-os-control-center to manage diskless garos endpoints on NixOS.",
        contact(name = "Kryonix", url = "https://kryonix.local"),
        license(name = "MIT/Apache-2.0"),
    ),
    paths(
        crate::handlers::auth::login,
        crate::handlers::auth::refresh,
        crate::handlers::auth::logout,
        crate::handlers::auth::me,
        crate::handlers::nodes::list_nodes,
        crate::handlers::nodes::get_node,
        crate::handlers::nodes::wol_node,
        crate::handlers::nodes::reboot_node,
        crate::handlers::nodes::bulk_wol,
        crate::handlers::nodes::bulk_shutdown,
        crate::handlers::nodes::bulk_reimage,
        crate::handlers::nodes::node_stats,
        crate::handlers::health::health,
        crate::handlers::health::ready,
    ),
    components(
        schemas(
            crate::domain::user::LoginRequest,
            crate::domain::user::LoginResponse,
            crate::domain::user::UserBrief,
            crate::domain::user::UserCreate,
            crate::domain::user::UserUpdate,
            crate::domain::user::UserView,
            crate::domain::user::UserStats,
            crate::domain::user::RefreshRequest,
            crate::domain::user::PasswordResetRequest,
            crate::domain::user::QuotaUpdateRequest,
            crate::domain::user::StatusUpdateRequest,
            crate::domain::node::NetbootDevice,
            crate::domain::node::HeartbeatRequest,
            crate::domain::node::ReimageRequest,
            crate::domain::node::BulkMacRequest,
            crate::domain::node::BulkReimageRequest,
            crate::domain::node::NodeStats,
            crate::domain::node::WolResult,
            crate::domain::node::BulkActionResult,
            crate::domain::node::BulkActionItem,
            crate::domain::image::ImageView,
            crate::domain::image::ImageCreate,
            crate::domain::image::ImageUpdate,
            crate::domain::image::ImageBuildStatus,
            crate::domain::image::ImageVersion,
            crate::domain::image::ImageDiff,
            crate::domain::firewall::FirewallRuleView,
            crate::domain::firewall::FirewallRuleCreate,
            crate::domain::firewall::FirewallRuleUpdate,
            crate::domain::firewall::FirewallRulePreview,
            crate::domain::firewall::ConnectionEntry,
            crate::domain::firewall::PanicStatus,
            crate::domain::storage::StoragePool,
            crate::domain::storage::ScrubStatus,
            crate::domain::storage::Snapshot,
            crate::domain::storage::SnapshotCreate,
            crate::domain::storage::Drive,
            crate::domain::storage::NfsExport,
            crate::domain::storage::NfsExportSpec,
            crate::domain::service::ServiceView,
            crate::domain::service::ServiceHealth,
            crate::domain::service::LogLine,
            crate::domain::audit::AuditEntry,
            crate::domain::audit::AuditQuery,
            crate::domain::audit::AuditStats,
            crate::domain::audit::ActivityEvent,
            crate::error::ErrorBody,
            crate::error::ErrorDetail,
            crate::error::FieldError,
            crate::handlers::auth::MeResponse,
            crate::handlers::health::VersionInfo,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "nodes", description = "Diskless stations (PXE)"),
        (name = "users", description = "User accounts"),
        (name = "images", description = "PXE/NixOS images"),
        (name = "firewall", description = "NFTables rules & panic"),
        (name = "storage", description = "BTRFS pools, snapshots, NFS"),
        (name = "services", description = "systemd service control"),
        (name = "metrics", description = "Live metrics, series, SLA"),
        (name = "activity", description = "Activity feed"),
        (name = "audit", description = "Audit log + export"),
        (name = "system", description = "Health, ready, docs, version"),
    ),
)]
pub struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    let mut o = ApiDoc::openapi();
    let components = o
        .components
        .get_or_insert_with(utoipa::openapi::Components::new);
    components.add_security_scheme(
        "bearer",
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .build(),
        ),
    );
    o
}
