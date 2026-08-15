//! `garos-backend` CLI entrypoint.

use clap::{Parser, Subcommand};
use garos_backend::api::build_router;
use garos_backend::auth::jwt::{AuthUser, JwtService, Role};
use garos_backend::config::Settings;
use garos_backend::db::pool::{build_pool, run_migrations};
use garos_backend::db::repositories::audit::AuditRepo;
use garos_backend::db::repositories::firewall::FirewallRepo;
use garos_backend::db::repositories::images::ImageRepo;
use garos_backend::db::repositories::nodes::NodeRepo;
use garos_backend::db::repositories::services::ServiceHealthRepo;
use garos_backend::db::repositories::storage::StorageRepo;
use garos_backend::db::repositories::users::UserRepo;
use garos_backend::integrations::btrfs::{Btrfs, BtrfsIntegration};
use garos_backend::integrations::journald::JournaldIntegration;
use garos_backend::integrations::nftables::{Nftables, NftablesIntegration};
use garos_backend::integrations::nix::{Nix, NixIntegration};
use garos_backend::integrations::pxe::{Pxe, PxeIntegration};
use garos_backend::integrations::samba::{Samba, SambaIntegration};
use garos_backend::integrations::systemd::SystemdIntegration;
use garos_backend::integrations::wol::{Wol, WolIntegration};
use garos_backend::realtime::hub::RealtimeHub;
use garos_backend::services::audit_service::AuditService;
use garos_backend::services::firewall_service::FirewallService;
use garos_backend::services::image_service::ImageService;
use garos_backend::services::node_service::NodeService;
use garos_backend::services::service_manager::ServiceManager;
use garos_backend::services::storage_service::StorageService;
use garos_backend::services::user_service::UserService;
use garos_backend::state::AppState;
use garos_backend::telemetry;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "garos-backend", version, about = "garos management API server")]
struct Cli {
    /// Optional environment name (loads `config/{env}.toml` overrides).
    #[arg(long, global = true, default_value = "dev")]
    env: String,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run the HTTP server (default).
    Serve,
    /// Run embedded migrations.
    Migrate,
    /// Generate a JWT for testing.
    GenJwt {
        user_id: String,
        #[arg(long, default_value = "admin")]
        role: String,
    },
    /// Hash a password with Argon2id.
    GenPassword { plain: String },
    /// User management sub-commands.
    User {
        #[command(subcommand)]
        cmd: UserCmd,
    },
    /// Integration self-tests.
    IntegrationTest,
}

#[derive(Subcommand, Debug)]
enum UserCmd {
    Create {
        username: String,
        #[arg(long)]
        password: String,
        #[arg(long, default_value = "operator")]
        role: String,
    },
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let settings = Arc::new(Settings::load_env(&cli.env).unwrap_or_else(|e| {
        eprintln!("config error: {e}");
        std::process::exit(1);
    }));

    let _telemetry_guard = telemetry::init(&settings)?;
    let pool = build_pool(&settings.database).await?;
    if settings.database.run_migrations {
        run_migrations(&pool).await?;
    }
    let jwt = Arc::new(JwtService::new(settings.auth.clone())?);

    match cli.cmd {
        Cmd::Serve => serve(settings, pool, jwt).await,
        Cmd::Migrate => {
            info!("migrations applied");
            Ok(())
        }
        Cmd::GenJwt { user_id, role } => {
            let user = AuthUser {
                id: uuid::Uuid::parse_str(&user_id)
                    .unwrap_or_else(|_| uuid::Uuid::now_v7()),
                username: "cli".into(),
                role: Role::from_str(&role).unwrap_or(Role::Admin),
            };
            let pair = jwt.issue_pair(&user)?;
            println!("{}", pair.access_token);
            Ok(())
        }
        Cmd::GenPassword { plain } => {
            let h = garos_backend::auth::password::hash_password(&plain, settings.auth.argon2_cost)?;
            println!("{h}");
            Ok(())
        }
        Cmd::User { cmd } => match cmd {
            UserCmd::Create {
                username,
                password,
                role,
            } => {
                let repo = UserRepo::new(pool.clone());
                let svc = UserService::new(repo, jwt.clone());
                let req = garos_backend::domain::user::UserCreate {
                    username,
                    email: None,
                    display_name: None,
                    password,
                    role,
                    samba_dn: None,
                };
                let row = svc.create(req).await?;
                println!("created user: {} ({})", row.username, row.id());
                Ok(())
            }
            UserCmd::List => {
                let repo = UserRepo::new(pool.clone());
                let rows = repo
                    .list(None, None, None, 100, 0)
                    .await
                    .map_err(to_anyhow)?;
                for r in rows {
                    println!("{} {} {} {}", r.id, r.username, r.role, r.status);
                }
                Ok(())
            }
        },
        Cmd::IntegrationTest => integration_test(settings, pool).await,
    }
}

fn to_anyhow<E: std::fmt::Display>(e: E) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

async fn serve(
    settings: Arc<Settings>,
    pool: garos_backend::db::pool::DbPool,
    jwt: Arc<JwtService>,
) -> anyhow::Result<()> {
    let user_repo = UserRepo::new(pool.clone());
    let node_repo = NodeRepo::new(pool.clone());
    let image_repo = ImageRepo::new(pool.clone());
    let fw_repo = FirewallRepo::new(pool.clone());
    let storage_repo = StorageRepo::new(pool.clone());
    let audit_repo = AuditRepo::new(pool.clone());
    let health_repo = ServiceHealthRepo::new(pool.clone());

    let realtime = RealtimeHub::new(2048);
    let samba = Arc::new(SambaIntegration::new(settings.integrations.samba.clone(), settings.features.mock_integrations));
    let nix = Arc::new(NixIntegration::new(settings.integrations.nix.clone(), settings.features.mock_integrations));
    let btrfs = Arc::new(BtrfsIntegration::new(settings.integrations.btrfs.clone(), settings.features.mock_integrations));
    let nft = Arc::new(NftablesIntegration::new(settings.integrations.nftables.clone(), settings.features.mock_integrations));
    let systemd = Arc::new(SystemdIntegration::new(settings.integrations.systemd.clone(), settings.features.mock_integrations));
    let journald = Arc::new(JournaldIntegration::new(settings.integrations.journald.clone(), settings.features.mock_integrations));
    let wol = Arc::new(WolIntegration::new(settings.integrations.wol.clone(), settings.features.mock_integrations));
    let pxe = Arc::new(PxeIntegration::new(settings.integrations.pxe.clone(), settings.features.mock_integrations));

    let user_svc = UserService::new(user_repo, jwt.clone());
    let audit_svc = AuditService::new(audit_repo.clone());
    let node_svc = NodeService::new(node_repo, audit_repo.clone(), wol.clone(), pxe.clone(), nix.clone(), realtime.clone());
    let image_svc = ImageService::new(image_repo, audit_repo.clone(), pxe.clone(), nix.clone(), realtime.clone());
    let fw_svc = FirewallService::new(fw_repo, audit_repo.clone(), nft.clone(), realtime.clone());
    let storage_svc = StorageService::new(storage_repo, audit_repo.clone(), btrfs.clone());
    let svc_mgr = ServiceManager::new(systemd.clone(), health_repo);

    let state = AppState::new(
        settings.clone(),
        pool.clone(),
        jwt.clone(),
        realtime.clone(),
        user_svc,
        node_svc,
        image_svc,
        fw_svc,
        storage_svc,
        svc_mgr,
        audit_svc,
    );

    let app = build_router(state.clone(), &settings);
    let addr: SocketAddr = settings.server.socket_addr();
    info!(%addr, "starting garos-backend");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let guard = settings.server.graceful_shutdown_secs;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(guard))
        .await?;
    Ok(())
}

async fn shutdown_signal(graceful_secs: u64) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("ctrl-c received, shutting down"),
        _ = term => info!("SIGTERM received, shutting down"),
    }
    tokio::time::sleep(std::time::Duration::from_secs(graceful_secs)).await;
}

async fn integration_test(
    settings: Arc<Settings>,
    _pool: garos_backend::db::pool::DbPool,
) -> anyhow::Result<()> {
    println!("== Integration self-test (mock={}) ==", settings.features.mock_integrations);
    let samba = SambaIntegration::new(settings.integrations.samba.clone(), settings.features.mock_integrations);
    let nix = NixIntegration::new(settings.integrations.nix.clone(), settings.features.mock_integrations);
    let btrfs = BtrfsIntegration::new(settings.integrations.btrfs.clone(), settings.features.mock_integrations);
    let nft = NftablesIntegration::new(settings.integrations.nftables.clone(), settings.features.mock_integrations);
    let wol = WolIntegration::new(settings.integrations.wol.clone(), settings.features.mock_integrations);
    let pxe = PxeIntegration::new(settings.integrations.pxe.clone(), settings.features.mock_integrations);

    println!("samba: {:?}", samba.domain_info().await.map_err(to_anyhow)?);
    println!("nix:   {:?}", nix.nix_build(".#test").await.map_err(to_anyhow)?);
    println!("btrfs: {} pools", btrfs.pools().await.map_err(to_anyhow)?.len());
    println!("nft:   {}", nft.list_ruleset().await.map_err(to_anyhow)?.len());
    println!("wol:   {}", wol.send("AA:BB:CC:DD:EE:FF").await.map_err(to_anyhow)?.mac);
    println!("pxe:   {} bytes", pxe.render_menu(&[], &[]).await.map_err(to_anyhow)?.len());
    println!("All checks passed.");
    Ok(())
}
