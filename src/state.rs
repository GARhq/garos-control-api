//! Shared application state.

use crate::auth::jwt::JwtService;
use crate::config::Settings;
use crate::db::pool::DbPool;
use crate::realtime::hub::RealtimeHub;
use crate::services::audit_service::AuditService;
use crate::services::firewall_service::FirewallService;
use crate::services::image_service::ImageService;
use crate::services::node_service::NodeService;
use crate::services::service_manager::ServiceManager;
use crate::services::storage_service::StorageService;
use crate::services::user_service::UserService;
use std::sync::Arc;

/// Cheap-to-clone handle to all services.
#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Settings>,
    pub db: DbPool,
    pub jwt: Arc<JwtService>,
    pub realtime: RealtimeHub,
    pub users: Arc<UserService>,
    pub nodes: Arc<NodeService>,
    pub images: Arc<ImageService>,
    pub firewall: Arc<FirewallService>,
    pub storage: Arc<StorageService>,
    pub services: Arc<ServiceManager>,
    pub audit: Arc<AuditService>,
    pub idempotency: Arc<crate::middleware::idempotency::IdempotencyStore>,
    pub metrics: Arc<crate::metrics::Metrics>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("settings", &"…")
            .field("db", &"DbPool")
            .field("jwt", &"JwtService")
            .field("realtime", &"RealtimeHub")
            .finish()
    }
}

impl AppState {
    pub fn new(
        settings: Arc<Settings>,
        db: DbPool,
        jwt: Arc<JwtService>,
        realtime: RealtimeHub,
        users: UserService,
        nodes: NodeService,
        images: ImageService,
        firewall: FirewallService,
        storage: StorageService,
        services: ServiceManager,
        audit: AuditService,
    ) -> Self {
        let idempotency = Arc::new(crate::middleware::idempotency::IdempotencyStore::new(
            settings.auth.idempotency_ttl(),
        ));
        let metrics = Arc::new(crate::metrics::Metrics::new().expect("metrics registry"));
        Self {
            settings,
            db,
            jwt,
            realtime,
            users: Arc::new(users),
            nodes: Arc::new(nodes),
            images: Arc::new(images),
            firewall: Arc::new(firewall),
            storage: Arc::new(storage),
            services: Arc::new(services),
            audit: Arc::new(audit),
            idempotency,
            metrics,
        }
    }
}
