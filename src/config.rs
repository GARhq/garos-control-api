//! Application configuration loaded from TOML files + env vars.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Top-level settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub server: ServerSettings,
    #[serde(default)]
    pub database: DatabaseSettings,
    #[serde(default)]
    pub auth: AuthSettings,
    #[serde(default)]
    pub integrations: IntegrationsSettings,
    #[serde(default)]
    pub ratelimit: RateLimitSettings,
    #[serde(default)]
    pub telemetry: TelemetrySettings,
    #[serde(default)]
    pub cors: CorsSettings,
    #[serde(default)]
    pub features: FeaturesSettings,
    #[serde(default)]
    pub logging: LoggingSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings {
    pub bind_addr: String,
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,
    #[serde(default = "default_body_size_limit")]
    pub body_size_limit: usize,
    #[serde(default = "default_graceful_shutdown_secs")]
    pub graceful_shutdown_secs: u64,
}

impl ServerSettings {
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.bind_addr, self.port)
            .parse()
            .expect("bind_addr/port must form a valid socket address")
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseSettings {
    /// Database URL. Default points to a persistent file under
    /// `/var/lib/garos/garos.db` so a deploy without override survives
    /// restarts. Override via env `DATABASE__URL=sqlite://...`.
    /// SECURITY (AURA-20260813-014): must never resolve to `sqlite::memory:`
    /// in non-test paths — see [`DatabaseSettings::reject_in_memory`].
    #[serde(default = "default_database_url")]
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    #[serde(default = "default_acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,
    #[serde(default = "default_run_migrations")]
    pub run_migrations: bool,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            acquire_timeout_secs: default_acquire_timeout_secs(),
            run_migrations: default_run_migrations(),
        }
    }
}

impl DatabaseSettings {
    pub fn acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.acquire_timeout_secs)
    }

    /// Fail-fast guard for catastrophic defaults. `sqlite::memory:` wipes
    /// the entire database on every process restart with no warning.
    /// Tests opt in to the in-memory behavior explicitly via
    /// `GAROS_ALLOW_IN_MEMORY=1` in their setup.
    /// Reference: 02-Contrato/api/AURA-20260813-014 (P0).
    pub fn reject_in_memory(&self) -> Result<(), String> {
        self.reject_in_memory_with(std::env::var("GAROS_ALLOW_IN_MEMORY").ok().as_deref())
    }

    /// Pure check (no env access) — same logic, takes the env value as
    /// a parameter. Useful in tests where global env mutation is undesirable.
    pub fn reject_in_memory_with(&self, allow_in_memory: Option<&str>) -> Result<(), String> {
        if self.url.trim() == "sqlite::memory:" {
            if allow_in_memory.is_some() {
                return Ok(());
            }
            return Err(format!(
                "database.url resolves to 'sqlite::memory:' which wipes all data on \
                 every restart. Set DATABASE__URL=sqlite:///var/lib/garos/garos.db \
                 (or any persistent file) before booting. To run with in-memory \
                 explicitly, set GAROS_ALLOW_IN_MEMORY=1. (AURA-20260813-014)"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthSettings {
    pub jwt_secret: Option<String>,
    pub jwt_private_key_path: Option<PathBuf>,
    pub jwt_public_key_path: Option<PathBuf>,
    #[serde(default = "default_jwt_issuer")]
    pub jwt_issuer: String,
    #[serde(default = "default_jwt_audience")]
    pub jwt_audience: String,
    #[serde(default = "default_access_ttl")]
    pub access_token_ttl_secs: u64,
    #[serde(default = "default_refresh_ttl")]
    pub refresh_token_ttl_secs: u64,
    #[serde(default = "default_argon2_cost")]
    pub argon2_cost: u32,
    #[serde(default = "default_idempotency_ttl")]
    pub idempotency_ttl_secs: u64,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            jwt_secret: None,
            jwt_private_key_path: None,
            jwt_public_key_path: None,
            jwt_issuer: default_jwt_issuer(),
            jwt_audience: default_jwt_audience(),
            access_token_ttl_secs: default_access_ttl(),
            refresh_token_ttl_secs: default_refresh_ttl(),
            argon2_cost: default_argon2_cost(),
            idempotency_ttl_secs: default_idempotency_ttl(),
        }
    }
}

impl AuthSettings {
    pub fn access_ttl(&self) -> Duration {
        Duration::from_secs(self.access_token_ttl_secs)
    }

    pub fn refresh_ttl(&self) -> Duration {
        Duration::from_secs(self.refresh_token_ttl_secs)
    }

    pub fn idempotency_ttl(&self) -> Duration {
        Duration::from_secs(self.idempotency_ttl_secs)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IntegrationsSettings {
    pub nix: NixSettings,
    pub samba: SambaSettings,
    pub btrfs: BtrfsSettings,
    pub nftables: NftablesSettings,
    pub systemd: SystemdSettings,
    pub journald: JournaldSettings,
    pub wol: WolSettings,
    pub pxe: PxeSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NixSettings {
    pub binary_path: PathBuf,
    pub store_path: Option<PathBuf>,
    #[serde(default = "default_build_timeout")]
    pub build_timeout_secs: u64,
    #[serde(default = "default_flake_dir")]
    pub flake_dir: PathBuf,
}

impl Default for NixSettings {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("/run/current-system/sw/bin/nix"),
            store_path: Some(PathBuf::from("/nix/store")),
            build_timeout_secs: default_build_timeout(),
            flake_dir: default_flake_dir(),
        }
    }
}

impl NixSettings {
    pub fn build_timeout(&self) -> Duration {
        Duration::from_secs(self.build_timeout_secs)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SambaSettings {
    pub realm: String,
    pub workgroup: String,
    pub dc_host: String,
    pub ldap_base_dn: String,
    pub ldap_bind_dn: String,
    pub ldap_bind_password_path: PathBuf,
    #[serde(default = "default_samba_use_tls")]
    pub use_tls: bool,
    #[serde(default = "default_samba_ldap_port")]
    pub ldap_port: u16,
}

impl Default for SambaSettings {
    fn default() -> Self {
        Self {
            realm: "KRYONIX.LOCAL".to_string(),
            workgroup: "KRYONIX".to_string(),
            dc_host: "127.0.0.1".to_string(),
            ldap_base_dn: "DC=kryonix,DC=local".to_string(),
            ldap_bind_dn: "cn=Administrator,CN=Users,DC=kryonix,DC=local".to_string(),
            ldap_bind_password_path: PathBuf::from("/etc/garos/samba-bind-pass"),
            use_tls: true,
            ldap_port: 636,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BtrfsSettings {
    pub mountpoint: PathBuf,
    #[serde(default = "default_scrub_path")]
    pub binary_path: PathBuf,
}

impl Default for BtrfsSettings {
    fn default() -> Self {
        Self {
            mountpoint: PathBuf::from("/srv/garos"),
            binary_path: default_scrub_path(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NftablesSettings {
    pub table_name: String,
    pub family: String,
    #[serde(default = "default_nft_binary")]
    pub binary_path: PathBuf,
}

impl Default for NftablesSettings {
    fn default() -> Self {
        Self {
            table_name: "garos".to_string(),
            family: "inet".to_string(),
            binary_path: default_nft_binary(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SystemdSettings {
    #[serde(default = "default_use_dbus")]
    pub use_dbus: bool,
    #[serde(default = "default_systemctl_binary")]
    pub systemctl_binary: PathBuf,
}

impl Default for SystemdSettings {
    fn default() -> Self {
        Self {
            use_dbus: true,
            systemctl_binary: default_systemctl_binary(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournaldSettings {
    pub remote_host: Option<String>,
    #[serde(default = "default_journald_max_bytes")]
    pub max_bytes: u64,
    #[serde(default = "default_journald_timeout")]
    pub timeout_secs: u64,
}

impl Default for JournaldSettings {
    fn default() -> Self {
        Self {
            remote_host: None,
            max_bytes: default_journald_max_bytes(),
            timeout_secs: default_journald_timeout(),
        }
    }
}

impl JournaldSettings {
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WolSettings {
    pub broadcast_addr: String,
    #[serde(default = "default_wol_port")]
    pub port: u16,
}

impl Default for WolSettings {
    fn default() -> Self {
        Self {
            broadcast_addr: "255.255.255.255".to_string(),
            port: default_wol_port(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PxeSettings {
    pub tftp_root: PathBuf,
    pub http_root: PathBuf,
    #[serde(default = "default_menu_timeout")]
    pub menu_timeout_secs: u64,
}

impl Default for PxeSettings {
    fn default() -> Self {
        Self {
            tftp_root: PathBuf::from("/srv/garos/tftp"),
            http_root: PathBuf::from("/srv/garos/http"),
            menu_timeout_secs: default_menu_timeout(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitSettings {
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,
    #[serde(default = "default_burst")]
    pub burst: u32,
}

impl Default for RateLimitSettings {
    fn default() -> Self {
        Self {
            requests_per_minute: default_rpm(),
            burst: default_burst(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetrySettings {
    pub otlp_endpoint: Option<String>,
    pub service_name: String,
    pub environment: String,
    #[serde(default)]
    pub sample_ratio: f64,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            otlp_endpoint: None,
            service_name: "garos-backend".to_string(),
            environment: "development".to_string(),
            sample_ratio: 1.0,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CorsSettings {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeaturesSettings {
    #[serde(default)]
    pub enable_postgres: bool,
    #[serde(default)]
    pub enable_otlp: bool,
    #[serde(default = "default_mock_integrations")]
    pub mock_integrations: bool,
    #[serde(default)]
    pub seed_admin: bool,
}

impl Default for FeaturesSettings {
    fn default() -> Self {
        Self {
            enable_postgres: false,
            enable_otlp: false,
            mock_integrations: default_mock_integrations(),
            seed_admin: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettings {
    #[serde(default = "default_log_format")]
    pub format: LogFormat,
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            format: default_log_format(),
            level: default_log_level(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

impl Settings {
    /// Load settings from `config/default.toml` (relative to current dir)
    /// plus an optional `config/{env}.toml` (overrides) and `GAROS_*` env vars.
    pub fn load() -> Result<Self, config::ConfigError> {
        Self::load_from(None)
    }

    /// Load with an explicit environment name (e.g. `production`, `dev`).
    pub fn load_env(env: &str) -> Result<Self, config::ConfigError> {
        Self::load_from(Some(env))
    }

    /// Internal loader. `env` is the optional profile name; the env-var prefix
    /// is always `GAROS`.
    pub fn load_from(env: Option<&str>) -> Result<Self, config::ConfigError> {
        let _ = dotenvy::dotenv();

        let mut builder = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(true))
            // Profile-specific overrides
            .add_source(
                env.map(|e| config::File::with_name(&format!("config/{e}")).required(false))
                    .unwrap_or_else(|| config::File::with_name("config/never").required(false)),
            )
            .add_source(
                config::File::with_name("/etc/garos/config")
                    .required(false),
            )
            .add_source(
                config::Environment::with_prefix("GAROS")
                    .separator("__")
                    .try_parsing(true)
                    .list_separator(","),
            );

        let s: Settings = builder.build()?.try_deserialize()?;

        // SECURITY (AURA-20260813-014): fail-fast if in-memory DB slips through.
        s.database.reject_in_memory().map_err(|e| {
            config::ConfigError::Message(format!("[AURA-20260813-014] {}", e))
        })?;

        Ok(s)
    }
}

fn default_workers() -> usize {
    num_cpus_or(2)
}

fn num_cpus_or(n: usize) -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(n)
}

fn default_request_timeout_secs() -> u64 {
    30
}
fn default_body_size_limit() -> usize {
    2 * 1024 * 1024 // 2 MiB
}
fn default_graceful_shutdown_secs() -> u64 {
    15
}
fn default_max_connections() -> u32 {
    16
}
fn default_min_connections() -> u32 {
    2
}
fn default_database_url() -> String {
    // SECURITY (AURA-20260813-014): default points to a persistent file,
    // never to in-memory. Operator can override via DATABASE__URL.
    "sqlite:///var/lib/garos/garos.db?mode=rwc".to_string()
}
fn default_acquire_timeout_secs() -> u64 {
    5
}
fn default_run_migrations() -> bool {
    true
}
fn default_jwt_issuer() -> String {
    "garos-control-api".to_string()
}
fn default_jwt_audience() -> String {
    "garos-control-center".to_string()
}
fn default_access_ttl() -> u64 {
    900 // 15 min
}
fn default_refresh_ttl() -> u64 {
    60 * 60 * 24 * 7 // 1 week
}
fn default_argon2_cost() -> u32 {
    65540 // OWASP recommended m-cost for argon2id
}
fn default_idempotency_ttl() -> u64 {
    60 * 60 * 24 // 24h
}
fn default_build_timeout() -> u64 {
    60 * 60 // 1h
}
fn default_flake_dir() -> PathBuf {
    PathBuf::from("/etc/garos/flake")
}
fn default_samba_use_tls() -> bool {
    true
}
fn default_samba_ldap_port() -> u16 {
    636
}
fn default_scrub_path() -> PathBuf {
    PathBuf::from("/run/current-system/sw/bin/btrfs")
}
fn default_nft_binary() -> PathBuf {
    PathBuf::from("/run/current-system/sw/bin/nft")
}
fn default_use_dbus() -> bool {
    true
}
fn default_systemctl_binary() -> PathBuf {
    PathBuf::from("/run/current-system/sw/bin/systemctl")
}
fn default_journald_max_bytes() -> u64 {
    1024 * 1024 * 16
}
fn default_journald_timeout() -> u64 {
    30
}
fn default_wol_port() -> u16 {
    9
}
fn default_menu_timeout() -> u64 {
    10
}
fn default_rpm() -> u32 {
    600
}
fn default_burst() -> u32 {
    60
}
fn default_mock_integrations() -> bool {
    true
}
fn default_log_format() -> LogFormat {
    LogFormat::Pretty
}
fn default_log_level() -> String {
    "info,garos_backend=debug,tower_http=info,sqlx=warn".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal_settings() {
        let toml = r#"
            [server]
            bind_addr = "0.0.0.0"
            port = 8080
            [database]
            url = "sqlite::memory:"
            [auth]
        "#;
        let s: Settings = toml::from_str(toml).unwrap();
        assert_eq!(s.server.port, 8080);
        assert_eq!(s.server.workers, default_workers());
        assert!(s.features.mock_integrations);
    }

    #[test]
    fn reject_in_memory_blocks_in_memory_url() {
        // Use the pure check to avoid touching the global env.
        let dbs = DatabaseSettings {
            url: "sqlite::memory:".to_string(),
            ..toml::from_str::<Settings>(
                r#"[server]
                   bind_addr = "0.0.0.0"
                   port = 8080
                   [auth]
                "#,
            )
            .unwrap()
            .database
        };
        let result = dbs.reject_in_memory_with(None);
        assert!(result.is_err(), "expected reject_in_memory to fail without GAROS_ALLOW_IN_MEMORY");
        assert!(result.unwrap_err().contains("AURA-20260813-014"));
    }

    #[test]
    fn reject_in_memory_allows_with_opt_in() {
        let dbs = DatabaseSettings {
            url: "sqlite::memory:".to_string(),
            ..toml::from_str::<Settings>(
                r#"[server]
                   bind_addr = "0.0.0.0"
                   port = 8080
                   [auth]
                "#,
            )
            .unwrap()
            .database
        };
        assert!(dbs.reject_in_memory_with(Some("1")).is_ok());
    }

    #[test]
    fn reject_in_memory_allows_persistent_url() {
        let dbs = DatabaseSettings {
            url: "sqlite:///var/lib/garos/garos.db?mode=rwc".to_string(),
            ..toml::from_str::<Settings>(
                r#"[server]
                   bind_addr = "0.0.0.0"
                   port = 8080
                   [auth]
                "#,
            )
            .unwrap()
            .database
        };
        assert!(dbs.reject_in_memory().is_ok());
    }

    #[test]
    fn socket_addr_is_parsed() {
        let s = ServerSettings {
            bind_addr: "127.0.0.1".to_string(),
            port: 9000,
            workers: 1,
            request_timeout_secs: 30,
            body_size_limit: 1024,
            graceful_shutdown_secs: 5,
        };
        assert_eq!(s.socket_addr().to_string(), "127.0.0.1:9000");
    }
}
