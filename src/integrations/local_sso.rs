//! Local SSO backend — replaces `SambaIntegration` for the K-010 Samba-free build.
//!
//! This is a **stub** for Fase 1 of K-010. The implementation that talks to the
//! real SSSD/PAM stack lives behind `LocalSsoBackend` and will be filled in
//! during Fase 2/3. For now, every method either:
//!
//!   1. Returns deterministic mock data (when `LocalSsoConfig::mock = true`),
//!   2. Or returns `AppError::ServiceUnavailable` to signal "not yet wired".
//!
//! The public API mirrors `crate::integrations::samba::Samba` so Fase 2/3 can
//! swap the import without changing every call-site.
//!
//! What this stub deliberately does **not** do:
//!   - It does NOT spawn `smbclient` or `samba-tool`.
//!   - It does NOT touch `Cargo.toml` (no new dependency added).
//!   - It does NOT delete `src/integrations/samba.rs` (Fase 2/3 responsibility).
//!
//! It does add the necessary type so that Fase 2/3 can `use LocalSsoBackend`
//! and replace the constructor in `main.rs` / `tests/common/mod.rs` with a
//! minimal patch.
use crate::config::SambaSettings;
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// User record returned by the Local SSO backend.
///
/// Structurally compatible with `samba::SambaUser` so Fase 2/3 callers can
/// migrate field-by-field. Field names are intentionally aligned with the
/// existing `SambaUser` (which is in turn a serialisable LDAP attribute bag).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSsoUser {
    pub dn: String,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub uid: Option<String>,
    pub gid_number: Option<String>,
    pub disabled: bool,
    pub last_logon: Option<String>,
    pub when_changed: Option<String>,
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSsoGroup {
    pub dn: String,
    pub name: String,
    pub gid: Option<String>,
    pub members: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSsoOu {
    pub dn: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSsoGpo {
    pub dn: String,
    pub display_name: String,
    pub flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDnsRecord {
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDomainInfo {
    pub realm: String,
    pub workgroup: String,
    pub dc_host: String,
    pub forest_level: u32,
    pub domain_level: u32,
    pub users: u32,
    pub groups: u32,
    pub computers: u32,
}

/// Trait contract.
///
/// The name `LocalSso` follows `fase-2-stub-rust.md`. The `Send + Sync`
/// bound is mandatory because callers use `Arc<dyn LocalSso>` in
/// `main.rs` (matches the existing `Arc<dyn Samba>` pattern).
#[async_trait]
pub trait LocalSso: Send + Sync {
    async fn list_users(&self) -> Result<Vec<LocalSsoUser>, AppError>;
    async fn get_user(&self, username: &str) -> Result<Option<LocalSsoUser>, AppError>;
    async fn create_user(
        &self,
        username: &str,
        password: &str,
        display_name: Option<&str>,
    ) -> Result<LocalSsoUser, AppError>;
    async fn update_user(
        &self,
        username: &str,
        display_name: Option<&str>,
        email: Option<&str>,
        disabled: bool,
    ) -> Result<LocalSsoUser, AppError>;
    async fn delete_user(&self, username: &str) -> Result<(), AppError>;
    async fn list_groups(&self) -> Result<Vec<LocalSsoGroup>, AppError>;
    async fn add_group_member(&self, group: &str, member_dn: &str) -> Result<(), AppError>;
    async fn list_ous(&self) -> Result<Vec<LocalSsoOu>, AppError>;
    async fn list_gpos(&self) -> Result<Vec<LocalSsoGpo>, AppError>;
    async fn list_dns_records(&self, zone: &str) -> Result<Vec<LocalDnsRecord>, AppError>;
    async fn domain_info(&self) -> Result<LocalDomainInfo, AppError>;
    async fn join_station(&self, hostname: &str, mac: &str) -> Result<(), AppError>;
    async fn leave_station(&self, hostname: &str) -> Result<(), AppError>;
}

/// Configuration struct — same shape as `SambaSettings` so Fase 2/3 can
/// pass the existing config section through.
#[derive(Debug, Clone)]
pub struct LocalSsoConfig {
    pub settings: SambaSettings,
    pub mock: bool,
}

impl LocalSsoConfig {
    pub fn from_samba_settings(settings: SambaSettings, mock: bool) -> Self {
        Self { settings, mock }
    }
}

#[derive(Default)]
struct MockState {
    users: Vec<LocalSsoUser>,
    groups: Vec<LocalSsoGroup>,
    ous: Vec<LocalSsoOu>,
    gpos: Vec<LocalSsoGpo>,
    seeded: bool,
}

/// Stub backend. Replaces `SambaIntegration` in Fase 2/3.
pub struct LocalSsoBackend {
    cfg: LocalSsoConfig,
    mock_data: Mutex<MockState>,
}

impl LocalSsoBackend {
    pub fn new(cfg: LocalSsoConfig) -> Self {
        Self {
            cfg,
            mock_data: Mutex::new(MockState::default()),
        }
    }

    /// Back-compat constructor matching `SambaIntegration::new(settings, mock)`.
    pub fn from_samba(settings: SambaSettings, mock: bool) -> Self {
        Self::new(LocalSsoConfig::from_samba_settings(settings, mock))
    }

    fn ensure_seeded(&self) {
        let mut g = self.mock_data.lock();
        if g.seeded {
            return;
        }
        g.seeded = true;
        let base_dn = &self.cfg.settings.ldap_base_dn;
        g.users = vec![
            LocalSsoUser {
                dn: format!("CN=Administrator,CN=Users,{base_dn}"),
                username: "Administrator".into(),
                display_name: Some("Domain Administrator".into()),
                email: None,
                uid: Some("500".into()),
                gid_number: Some("513".into()),
                disabled: false,
                last_logon: None,
                when_changed: None,
                groups: vec!["Domain Admins".into(), "Enterprise Admins".into()],
            },
            LocalSsoUser {
                dn: format!("CN=alice,OU=Users,{base_dn}"),
                username: "alice".into(),
                display_name: Some("Alice Anderson".into()),
                email: Some("alice@kryonix.local".into()),
                uid: Some("11001".into()),
                gid_number: Some("513".into()),
                disabled: false,
                last_logon: None,
                when_changed: None,
                groups: vec!["Domain Users".into()],
            },
        ];
        g.groups = vec![
            LocalSsoGroup {
                dn: format!("CN=Domain Admins,CN=Users,{base_dn}"),
                name: "Domain Admins".into(),
                gid: Some("512".into()),
                members: vec![format!("CN=Administrator,CN=Users,{base_dn}")],
                description: Some("Designated administrators".into()),
            },
            LocalSsoGroup {
                dn: format!("CN=Domain Users,CN=Users,{base_dn}"),
                name: "Domain Users".into(),
                gid: Some("513".into()),
                members: vec![format!("CN=alice,OU=Users,{base_dn}")],
                description: None,
            },
        ];
        g.ous = vec![LocalSsoOu {
            dn: format!("OU=Computers,{base_dn}"),
            name: "Computers".into(),
            description: Some("Default container for workstation accounts".into()),
        }];
        g.gpos = vec![LocalSsoGpo {
            dn: format!("CN={{B4B7E1AB-...}},CN=Policies,CN=System,{base_dn}"),
            display_name: "Default Domain Policy".into(),
            flags: 0,
        }];
    }

    /// Placeholder for the real PAM/SSSD conversation that Fase 2 will wire up.
    fn not_wired(&self, op: &str) -> AppError {
        AppError::ServiceUnavailable(format!(
            "LocalSsoBackend::{op} is a K-010 stub — implementation lands in Fase 2"
        ))
    }
}

#[async_trait]
impl LocalSso for LocalSsoBackend {
    async fn list_users(&self) -> Result<Vec<LocalSsoUser>, AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            return Ok(self.mock_data.lock().users.clone());
        }
        // Fase 2 will replace this with SSSD NSS enumeration (`nsswitch`).
        Err(self.not_wired("list_users"))
    }

    async fn get_user(&self, username: &str) -> Result<Option<LocalSsoUser>, AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            let g = self.mock_data.lock();
            return Ok(g
                .users
                .iter()
                .find(|u| u.username.eq_ignore_ascii_case(username))
                .cloned());
        }
        Err(self.not_wired("get_user"))
    }

    async fn create_user(
        &self,
        username: &str,
        _password: &str,
        display_name: Option<&str>,
    ) -> Result<LocalSsoUser, AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            let mut g = self.mock_data.lock();
            if g.users.iter().any(|u| u.username == username) {
                return Err(AppError::Conflict(format!(
                    "user {username} already exists"
                )));
            }
            let u = LocalSsoUser {
                dn: format!("CN={username},CN=Users,{}", self.cfg.settings.ldap_base_dn),
                username: username.into(),
                display_name: display_name.map(str::to_string),
                email: Some(format!(
                    "{username}@{}",
                    self.cfg.settings.realm.to_lowercase()
                )),
                uid: None,
                gid_number: Some("513".into()),
                disabled: false,
                last_logon: None,
                when_changed: None,
                groups: vec!["Domain Users".into()],
            };
            g.users.push(u.clone());
            return Ok(u);
        }
        Err(self.not_wired("create_user"))
    }

    async fn update_user(
        &self,
        username: &str,
        display_name: Option<&str>,
        email: Option<&str>,
        disabled: bool,
    ) -> Result<LocalSsoUser, AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            let mut g = self.mock_data.lock();
            let u = g
                .users
                .iter_mut()
                .find(|u| u.username == username)
                .ok_or_else(|| AppError::NotFound(format!("user {username}")))?;
            if let Some(d) = display_name {
                u.display_name = Some(d.into());
            }
            if let Some(e) = email {
                u.email = Some(e.into());
            }
            u.disabled = disabled;
            return Ok(u.clone());
        }
        Err(self.not_wired("update_user"))
    }

    async fn delete_user(&self, username: &str) -> Result<(), AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            self.mock_data
                .lock()
                .users
                .retain(|u| u.username != username);
            return Ok(());
        }
        Err(self.not_wired("delete_user"))
    }

    async fn list_groups(&self) -> Result<Vec<LocalSsoGroup>, AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            return Ok(self.mock_data.lock().groups.clone());
        }
        Err(self.not_wired("list_groups"))
    }

    async fn add_group_member(&self, group: &str, member_dn: &str) -> Result<(), AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            let mut g = self.mock_data.lock();
            if let Some(gr) = g.groups.iter_mut().find(|g| g.name == group) {
                if !gr.members.contains(&member_dn.to_string()) {
                    gr.members.push(member_dn.into());
                }
            }
            return Ok(());
        }
        Err(self.not_wired("add_group_member"))
    }

    async fn list_ous(&self) -> Result<Vec<LocalSsoOu>, AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            return Ok(self.mock_data.lock().ous.clone());
        }
        Err(self.not_wired("list_ous"))
    }

    async fn list_gpos(&self) -> Result<Vec<LocalSsoGpo>, AppError> {
        if self.cfg.mock {
            self.ensure_seeded();
            return Ok(self.mock_data.lock().gpos.clone());
        }
        Err(self.not_wired("list_gpos"))
    }

    async fn list_dns_records(&self, _zone: &str) -> Result<Vec<LocalDnsRecord>, AppError> {
        if self.cfg.mock {
            return Ok(vec![
                LocalDnsRecord {
                    name: "dc1".into(),
                    record_type: "A".into(),
                    value: "10.0.0.10".into(),
                    ttl: 3600,
                },
                LocalDnsRecord {
                    name: "gc".into(),
                    record_type: "A".into(),
                    value: "10.0.0.10".into(),
                    ttl: 3600,
                },
            ]);
        }
        Err(self.not_wired("list_dns_records"))
    }

    async fn domain_info(&self) -> Result<LocalDomainInfo, AppError> {
        if self.cfg.mock {
            return Ok(LocalDomainInfo {
                realm: self.cfg.settings.realm.clone(),
                workgroup: self.cfg.settings.workgroup.clone(),
                dc_host: self.cfg.settings.dc_host.clone(),
                forest_level: 2016,
                domain_level: 2016,
                users: 42,
                groups: 12,
                computers: 28,
            });
        }
        Err(self.not_wired("domain_info"))
    }

    async fn join_station(&self, hostname: &str, _mac: &str) -> Result<(), AppError> {
        if self.cfg.mock {
            tracing::info!(target: "local_sso", %hostname, "[mock] join station");
            return Ok(());
        }
        Err(self.not_wired("join_station"))
    }

    async fn leave_station(&self, hostname: &str) -> Result<(), AppError> {
        if self.cfg.mock {
            tracing::info!(target: "local_sso", %hostname, "[mock] leave station");
            return Ok(());
        }
        Err(self.not_wired("leave_station"))
    }
}

/// Backwards-compatible adapter so Fase 2 can keep using `IntegrationKind::Samba`
/// callers by adding `LocalSsoBackend` behind the existing trait object.
///
/// This is **not** the canonical swap point — Fase 2/3 should remove
/// `IntegrationKind::Samba` and rename to `IntegrationKind::Ldap`. Until
/// then, the integration kind is unchanged.
pub fn integration_kind() -> IntegrationKind {
    IntegrationKind::Samba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_returns_admin() {
        let backend = LocalSsoBackend::from_samba(SambaSettings::default(), true);
        let users = backend.list_users().await.unwrap();
        assert!(users.iter().any(|u| u.username == "Administrator"));
        let admin = backend
            .get_user("Administrator")
            .await
            .unwrap()
            .expect("admin must exist");
        assert!(!admin.disabled);
    }

    #[tokio::test]
    async fn mock_create_and_get() {
        let backend = LocalSsoBackend::from_samba(SambaSettings::default(), true);
        backend
            .create_user("bob", "Hunter22x!", Some("Bob"))
            .await
            .unwrap();
        let bob = backend
            .get_user("bob")
            .await
            .unwrap()
            .expect("bob must exist");
        assert_eq!(bob.display_name.as_deref(), Some("Bob"));
    }

    #[tokio::test]
    async fn real_backend_reports_not_wired() {
        let backend = LocalSsoBackend::from_samba(SambaSettings::default(), false);
        let err = backend.list_users().await.unwrap_err();
        match err {
            AppError::ServiceUnavailable(msg) => {
                assert!(msg.contains("K-010 stub"), "unexpected message: {msg}");
            }
            other => panic!("expected ServiceUnavailable, got {other:?}"),
        }
    }
}
