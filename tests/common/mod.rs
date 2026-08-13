//! Test-app builder used by integration tests.

use garos_backend::auth::jwt::JwtService;
use garos_backend::config::Settings;
use garos_backend::db::pool::memory_pool;
use garos_backend::db::repositories::audit::AuditRepo;
use garos_backend::db::repositories::firewall::FirewallRepo;
use garos_backend::db::repositories::images::ImageRepo;
use garos_backend::db::repositories::nodes::NodeRepo;
use garos_backend::db::repositories::services::ServiceHealthRepo;
use garos_backend::db::repositories::storage::StorageRepo;
use garos_backend::db::repositories::users::UserRepo;
use garos_backend::integrations::btrfs::BtrfsIntegration;
use garos_backend::integrations::nftables::NftablesIntegration;
use garos_backend::integrations::nix::NixIntegration;
use garos_backend::integrations::pxe::PxeIntegration;
use garos_backend::integrations::samba::SambaIntegration;
use garos_backend::integrations::systemd::SystemdIntegration;
use garos_backend::integrations::wol::WolIntegration;
use garos_backend::realtime::hub::RealtimeHub;
use garos_backend::services::audit_service::AuditService;
use garos_backend::services::firewall_service::FirewallService;
use garos_backend::services::image_service::ImageService;
use garos_backend::services::node_service::NodeService;
use garos_backend::services::service_manager::ServiceManager;
use garos_backend::services::storage_service::StorageService;
use garos_backend::services::user_service::UserService;
use garos_backend::state::AppState;
use std::sync::Arc;

pub struct TestApp {
    pub state: AppState,
    pub pool: garos_backend::db::pool::DbPool,
}

impl TestApp {
    pub async fn new() -> Self {
        let pool = memory_pool().await.expect("memory pool");
        garos_backend::db::pool::run_migrations(&pool)
            .await
            .expect("migrations");
        let settings = Arc::new(Settings {
            server: garos_backend::config::ServerSettings {
                bind_addr: "127.0.0.1".into(),
                port: 0,
                workers: 1,
                request_timeout_secs: 5,
                body_size_limit: 1024 * 1024,
                graceful_shutdown_secs: 1,
            },
            database: garos_backend::config::DatabaseSettings {
                url: "sqlite::memory:".into(),
                max_connections: 1,
                min_connections: 1,
                acquire_timeout_secs: 5,
                run_migrations: true,
            },
            auth: garos_backend::config::AuthSettings {
                jwt_secret: Some("test-secret-32+chars-here-for-hs256-mode".into()),
                jwt_private_key_path: None,
                jwt_public_key_path: None,
                jwt_issuer: "garos-test".into(),
                jwt_audience: "garos-test-api".into(),
                access_token_ttl_secs: 60,
                refresh_token_ttl_secs: 3600,
                argon2_cost: 4096,
                idempotency_ttl_secs: 60,
            },
            integrations: Default::default(),
            ratelimit: Default::default(),
            telemetry: Default::default(),
            cors: Default::default(),
            features: Default::default(),
            logging: Default::default(),
        });
        let jwt = Arc::new(JwtService::new(settings.auth.clone()).unwrap());
        let user_repo = UserRepo::new(pool.clone());
        let node_repo = NodeRepo::new(pool.clone());
        let image_repo = ImageRepo::new(pool.clone());
        let fw_repo = FirewallRepo::new(pool.clone());
        let storage_repo = StorageRepo::new(pool.clone());
        let audit_repo = AuditRepo::new(pool.clone());
        let health_repo = ServiceHealthRepo::new(pool.clone());
        let realtime = RealtimeHub::new(64);
        let samba = Arc::new(SambaIntegration::new(settings.integrations.samba.clone(), true));
        let nix = Arc::new(NixIntegration::new(settings.integrations.nix.clone(), true));
        let btrfs = Arc::new(BtrfsIntegration::new(settings.integrations.btrfs.clone(), true));
        let nft = Arc::new(NftablesIntegration::new(settings.integrations.nftables.clone(), true));
        let systemd = Arc::new(SystemdIntegration::new(settings.integrations.systemd.clone(), true));
        let wol = Arc::new(WolIntegration::new(settings.integrations.wol.clone(), true));
        let pxe = Arc::new(PxeIntegration::new(settings.integrations.pxe.clone(), true));
        let state = AppState::new(
            settings.clone(),
            pool.clone(),
            jwt,
            realtime,
            UserService::new(user_repo, Arc::new(JwtService::new(settings.auth.clone()).unwrap())),
            NodeService::new(node_repo, audit_repo.clone(), wol, pxe.clone(), nix.clone(), RealtimeHub::new(8)),
            ImageService::new(image_repo, audit_repo.clone(), pxe, nix, RealtimeHub::new(8)),
            FirewallService::new(fw_repo, audit_repo.clone(), nft, RealtimeHub::new(8)),
            StorageService::new(storage_repo, audit_repo.clone(), btrfs),
            ServiceManager::new(systemd, health_repo),
            AuditService::new(audit_repo),
        );
        Self { state, pool }
    }

    pub async fn seed_admin(&self) -> uuid::Uuid {
        use garos_backend::domain::user::UserCreate;
        let req = UserCreate {
            username: "admin".into(),
            email: Some("admin@test.local".into()),
            display_name: Some("Admin".into()),
            password: "ChangeMe!2024".into(),
            role: "admin".into(),
            samba_dn: None,
        };
        let user = self
            .state
            .users
            .create(req)
            .await
            .expect("seed admin");
        user.id()
    }
}

pub mod fixtures {
    pub const ADMIN_USER: &str = "admin";
    pub const ADMIN_PASS: &str = "ChangeMe!2024";
}
