//! Samba AD integration via LDAP3.
//!
//! All operations are best-effort with exponential retry on transient errors.

use crate::config::SambaSettings;
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use ldap3::{LdapConnAsync, LdapConnSettings, Scope, SearchEntry};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SambaUser {
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
pub struct SambaGroup {
    pub dn: String,
    pub name: String,
    pub gid: Option<String>,
    pub members: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SambaOu {
    pub dn: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SambaGpo {
    pub dn: String,
    pub display_name: String,
    pub flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainInfo {
    pub realm: String,
    pub workgroup: String,
    pub dc_host: String,
    pub forest_level: u32,
    pub domain_level: u32,
    pub users: u32,
    pub groups: u32,
    pub computers: u32,
}

#[async_trait]
pub trait Samba: Send + Sync {
    async fn list_users(&self) -> Result<Vec<SambaUser>, AppError>;
    async fn get_user(&self, username: &str) -> Result<Option<SambaUser>, AppError>;
    async fn create_user(
        &self,
        username: &str,
        password: &str,
        display_name: Option<&str>,
    ) -> Result<SambaUser, AppError>;
    async fn update_user(
        &self,
        username: &str,
        display_name: Option<&str>,
        email: Option<&str>,
        disabled: bool,
    ) -> Result<SambaUser, AppError>;
    async fn delete_user(&self, username: &str) -> Result<(), AppError>;
    async fn list_groups(&self) -> Result<Vec<SambaGroup>, AppError>;
    async fn add_group_member(&self, group: &str, member_dn: &str) -> Result<(), AppError>;
    async fn list_ous(&self) -> Result<Vec<SambaOu>, AppError>;
    async fn list_gpos(&self) -> Result<Vec<SambaGpo>, AppError>;
    async fn list_dns_records(&self, zone: &str) -> Result<Vec<DnsRecord>, AppError>;
    async fn domain_info(&self) -> Result<DomainInfo, AppError>;
    async fn join_station(&self, hostname: &str, mac: &str) -> Result<(), AppError>;
    async fn leave_station(&self, hostname: &str) -> Result<(), AppError>;
}

pub struct SambaIntegration {
    settings: SambaSettings,
    mock: bool,
    mock_data: parking_lot::Mutex<MockState>,
}

#[derive(Default)]
struct MockState {
    users: Vec<SambaUser>,
    groups: Vec<SambaGroup>,
    ous: Vec<SambaOu>,
    gpos: Vec<SambaGpo>,
    seeded: bool,
}

impl SambaIntegration {
    pub fn new(settings: SambaSettings, mock: bool) -> Self {
        Self {
            settings,
            mock,
            mock_data: parking_lot::Mutex::new(MockState::default()),
        }
    }

    fn ensure_seeded(&self) {
        let mut g = self.mock_data.lock();
        if g.seeded {
            return;
        }
        g.seeded = true;
        g.users = vec![
            SambaUser {
                dn: format!("CN=Administrator,CN=Users,{}", self.settings.ldap_base_dn),
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
            SambaUser {
                dn: format!("CN=alice,OU=Users,{}", self.settings.ldap_base_dn),
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
            SambaGroup {
                dn: format!("CN=Domain Admins,CN=Users,{}", self.settings.ldap_base_dn),
                name: "Domain Admins".into(),
                gid: Some("512".into()),
                members: vec![format!(
                    "CN=Administrator,CN=Users,{}",
                    self.settings.ldap_base_dn
                )],
                description: Some("Designated administrators".into()),
            },
            SambaGroup {
                dn: format!("CN=Domain Users,CN=Users,{}", self.settings.ldap_base_dn),
                name: "Domain Users".into(),
                gid: Some("513".into()),
                members: vec![format!(
                    "CN=alice,OU=Users,{}",
                    self.settings.ldap_base_dn
                )],
                description: None,
            },
        ];
        g.ous = vec![SambaOu {
            dn: format!("OU=Computers,{}", self.settings.ldap_base_dn),
            name: "Computers".into(),
            description: Some("Default container for workstation accounts".into()),
        }];
        g.gpos = vec![SambaGpo {
            dn: format!("CN={{B4B7E1AB-...}},CN=Policies,CN=System,{}", self.settings.ldap_base_dn),
            display_name: "Default Domain Policy".into(),
            flags: 0,
        }];
    }

    async fn connect(&self) -> Result<ldap3::Ldap, AppError> {
        let password = std::fs::read_to_string(&self.settings.ldap_bind_password_path)
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Samba,
                message: format!("read bind password: {e}"),
            })?;
        let password = password.trim_end_matches('\n').to_string();
        let url = if self.settings.use_tls {
            format!("ldaps://{}:{}", self.settings.dc_host, self.settings.ldap_port)
        } else {
            format!("ldap://{}:{}", self.settings.dc_host, self.settings.ldap_port)
        };
        let settings = || LdapConnSettings::new().set_conn_timeout(Duration::from_secs(5));
        for attempt in 1u32..=3 {
            match LdapConnAsync::with_settings(settings(), &url).await {
                Ok((conn, mut ldap)) => {
                    let _ = conn.drive();
                    match ldap
                        .simple_bind(&self.settings.ldap_bind_dn, &password)
                        .await
                    {
                        Ok(r) if r.rc == 0 => return Ok(ldap),
                        Ok(r) => {
                            return Err(AppError::Integration {
                                kind: IntegrationKind::Samba,
                                message: format!("bind rc={}", r.rc),
                            })
                        }
                        Err(e) => {
                            if attempt == 3 {
                                return Err(AppError::from(e));
                            }
                        }
                    }
                }
                Err(e) => {
                    if attempt == 3 {
                        return Err(AppError::from(e));
                    }
                    sleep(Duration::from_millis(200 * (1 << (attempt - 1)))).await;
                }
            }
        }
        Err(AppError::Integration {
            kind: IntegrationKind::Samba,
            message: "exhausted retries".into(),
        })
    }
}

#[async_trait]
impl Samba for SambaIntegration {
    async fn list_users(&self) -> Result<Vec<SambaUser>, AppError> {
        if self.mock {
            self.ensure_seeded();
            return Ok(self.mock_data.lock().users.clone());
        }
        let mut ldap = self.connect().await?;
        let (rs, _res) = ldap
            .search(
                &self.settings.ldap_base_dn,
                Scope::Subtree,
                "(&(objectClass=user)(objectCategory=person))",
                vec![
                    "sAMAccountName",
                    "displayName",
                    "mail",
                    "uidNumber",
                    "gidNumber",
                    "memberOf",
                    "lastLogon",
                    "whenChanged",
                    "userAccountControl",
                ],
            )
            .await?
            .success()
            .map_err(AppError::from)?;
        let mut users = Vec::with_capacity(rs.len());
        for entry in rs {
            let e = SearchEntry::construct(entry);
            let username = e.attrs.get("sAMAccountName").and_then(|v| v.first().cloned()).unwrap_or_default();
            let uac: u32 = e
                .attrs
                .get("userAccountControl")
                .and_then(|v| v.first().and_then(|s| s.parse().ok()))
                .unwrap_or(0);
            users.push(SambaUser {
                dn: e.dn,
                username,
                display_name: e.attrs.get("displayName").and_then(|v| v.first().cloned()),
                email: e.attrs.get("mail").and_then(|v| v.first().cloned()),
                uid: e.attrs.get("uidNumber").and_then(|v| v.first().cloned()),
                gid_number: e.attrs.get("gidNumber").and_then(|v| v.first().cloned()),
                disabled: (uac & 2) != 0,
                last_logon: e.attrs.get("lastLogon").and_then(|v| v.first().cloned()),
                when_changed: e.attrs.get("whenChanged").and_then(|v| v.first().cloned()),
                groups: e
                    .attrs
                    .get("memberOf")
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|dn| {
                        dn.split(',')
                            .next()
                            .and_then(|c| c.strip_prefix("CN=").map(str::to_string))
                    })
                    .collect(),
            });
        }
        Ok(users)
    }

    async fn get_user(&self, username: &str) -> Result<Option<SambaUser>, AppError> {
        let users = self.list_users().await?;
        Ok(users.into_iter().find(|u| u.username.eq_ignore_ascii_case(username)))
    }

    async fn create_user(
        &self,
        username: &str,
        password: &str,
        display_name: Option<&str>,
    ) -> Result<SambaUser, AppError> {
        if self.mock {
            self.ensure_seeded();
            let mut g = self.mock_data.lock();
            if g.users.iter().any(|u| u.username == username) {
                return Err(AppError::Conflict(format!("user {username} already exists")));
            }
            let u = SambaUser {
                dn: format!("CN={username},CN=Users,{}", self.settings.ldap_base_dn),
                username: username.into(),
                display_name: display_name.map(str::to_string),
                email: Some(format!("{username}@{}", self.settings.realm.to_lowercase())),
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
        let mut ldap = self.connect().await?;
        let dn = format!("CN={username},CN=Users,{}", self.settings.ldap_base_dn);
        let pw = format!("\"{password}\"");
        let attrs: Vec<(&str, std::collections::HashSet<&str>)> = vec![
            ("objectClass", ["top", "person", "organizationalPerson", "user"].into_iter().collect()),
            ("sAMAccountName", [username].into_iter().collect()),
            ("userAccountControl", ["512"].into_iter().collect()),
            ("unicodePwd", [pw.as_str()].into_iter().collect()),
        ];
        ldap.add(&dn, attrs).await?.success()?;
        Ok(SambaUser {
            dn,
            username: username.into(),
            display_name: display_name.map(str::to_string),
            email: None,
            uid: None,
            gid_number: Some("513".into()),
            disabled: false,
            last_logon: None,
            when_changed: None,
            groups: vec!["Domain Users".into()],
        })
    }

    async fn update_user(
        &self,
        username: &str,
        display_name: Option<&str>,
        email: Option<&str>,
        disabled: bool,
    ) -> Result<SambaUser, AppError> {
        if self.mock {
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
        let u = self
            .get_user(username)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("user {username}")))?;
        let mut mods: Vec<ldap3::Mod<String>> = Vec::new();
        if let Some(d) = display_name {
            mods.push(ldap3::Mod::Replace("displayName".to_string(), std::collections::HashSet::from([d.to_string()])));
        }
        if let Some(e) = email {
            mods.push(ldap3::Mod::Replace("mail".to_string(), std::collections::HashSet::from([e.to_string()])));
        }
        let new_uac = if disabled { 514u32 } else { 512u32 };
        mods.push(ldap3::Mod::Replace("userAccountControl".to_string(), std::collections::HashSet::from([new_uac.to_string()])));
        let mut ldap = self.connect().await?;
        ldap.modify(&u.dn, mods).await?.success()?;
        self.get_user(username).await?.ok_or_else(|| {
            AppError::NotFound(format!("user {username} after modify"))
        })
    }

    async fn delete_user(&self, username: &str) -> Result<(), AppError> {
        if self.mock {
            self.ensure_seeded();
            let mut g = self.mock_data.lock();
            g.users.retain(|u| u.username != username);
            return Ok(());
        }
        let u = self
            .get_user(username)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("user {username}")))?;
        let mut ldap = self.connect().await?;
        ldap.delete(&u.dn).await?.success()?;
        Ok(())
    }

    async fn list_groups(&self) -> Result<Vec<SambaGroup>, AppError> {
        if self.mock {
            self.ensure_seeded();
            return Ok(self.mock_data.lock().groups.clone());
        }
        let mut ldap = self.connect().await?;
        let (rs, _) = ldap
            .search(
                &self.settings.ldap_base_dn,
                Scope::Subtree,
                "(objectClass=group)",
                vec!["cn", "gidNumber", "member", "description"],
            )
            .await?
            .success()
            .map_err(AppError::from)?;
        let mut groups = Vec::with_capacity(rs.len());
        for entry in rs {
            let e = SearchEntry::construct(entry);
            groups.push(SambaGroup {
                dn: e.dn,
                name: e.attrs.get("cn").and_then(|v| v.first().cloned()).unwrap_or_default(),
                gid: e.attrs.get("gidNumber").and_then(|v| v.first().cloned()),
                members: e.attrs.get("member").cloned().unwrap_or_default(),
                description: e.attrs.get("description").and_then(|v| v.first().cloned()),
            });
        }
        Ok(groups)
    }

    async fn add_group_member(&self, group: &str, member_dn: &str) -> Result<(), AppError> {
        if self.mock {
            self.ensure_seeded();
            let mut g = self.mock_data.lock();
            if let Some(gr) = g.groups.iter_mut().find(|g| g.name == group) {
                if !gr.members.contains(&member_dn.to_string()) {
                    gr.members.push(member_dn.into());
                }
            }
            return Ok(());
        }
        let groups = self.list_groups().await?;
        let gr = groups
            .into_iter()
            .find(|g| g.name == group)
            .ok_or_else(|| AppError::NotFound(format!("group {group}")))?;
        let mut ldap = self.connect().await?;
        let mods: Vec<ldap3::Mod<String>> = vec![
            ldap3::Mod::Add(
                "member".to_string(),
                std::collections::HashSet::from([member_dn.to_string()]),
            ),
        ];
        ldap.modify(&gr.dn, mods).await?.success()?;
        Ok(())
    }

    async fn list_ous(&self) -> Result<Vec<SambaOu>, AppError> {
        if self.mock {
            self.ensure_seeded();
            return Ok(self.mock_data.lock().ous.clone());
        }
        let mut ldap = self.connect().await?;
        let (rs, _) = ldap
            .search(
                &self.settings.ldap_base_dn,
                Scope::Subtree,
                "(objectClass=organizationalUnit)",
                vec!["ou", "description"],
            )
            .await?
            .success()
            .map_err(AppError::from)?;
        let mut ous = Vec::with_capacity(rs.len());
        for entry in rs {
            let e = SearchEntry::construct(entry);
            ous.push(SambaOu {
                dn: e.dn,
                name: e.attrs.get("ou").and_then(|v| v.first().cloned()).unwrap_or_default(),
                description: e.attrs.get("description").and_then(|v| v.first().cloned()),
            });
        }
        Ok(ous)
    }

    async fn list_gpos(&self) -> Result<Vec<SambaGpo>, AppError> {
        if self.mock {
            self.ensure_seeded();
            return Ok(self.mock_data.lock().gpos.clone());
        }
        let mut ldap = self.connect().await?;
        let (rs, _) = ldap
            .search(
                &format!("CN=Policies,CN=System,{}", self.settings.ldap_base_dn),
                Scope::Subtree,
                "(objectClass=groupPolicyContainer)",
                vec!["displayName", "flags"],
            )
            .await?
            .success()
            .map_err(AppError::from)?;
        let mut gpos = Vec::with_capacity(rs.len());
        for entry in rs {
            let e = SearchEntry::construct(entry);
            gpos.push(SambaGpo {
                dn: e.dn,
                display_name: e
                    .attrs
                    .get("displayName")
                    .and_then(|v| v.first().cloned())
                    .unwrap_or_default(),
                flags: e
                    .attrs
                    .get("flags")
                    .and_then(|v| v.first().and_then(|s| s.parse().ok()))
                    .unwrap_or(0),
            });
        }
        Ok(gpos)
    }

    async fn list_dns_records(&self, zone: &str) -> Result<Vec<DnsRecord>, AppError> {
        if self.mock {
            return Ok(vec![
                DnsRecord {
                    name: "dc1".into(),
                    record_type: "A".into(),
                    value: "10.0.0.10".into(),
                    ttl: 3600,
                },
                DnsRecord {
                    name: "gc".into(),
                    record_type: "A".into(),
                    value: "10.0.0.10".into(),
                    ttl: 3600,
                },
                DnsRecord {
                    name: "@".into(),
                    record_type: "SOA".into(),
                    value: format!("dc1.{} hostmaster.{}.", self.settings.realm, self.settings.realm),
                    ttl: 3600,
                },
            ])
            .map(|mut v| {
                v.iter_mut().for_each(|_| {});
                v
            });
        }
        let mut ldap = self.connect().await?;
        let base = format!(
            "DC={},DC={}",
            zone.split('.').next().unwrap_or("dns"),
            zone.rsplit('.').next().unwrap_or("local"),
        );
        let (rs, _) = ldap
            .search(&base, Scope::Subtree, "(objectClass=dnsNode)", vec!["dnsRecord"])
            .await?
            .success()
            .map_err(AppError::from)?;
        let mut recs = Vec::with_capacity(rs.len());
        for entry in rs {
            let e = SearchEntry::construct(entry);
            let name = e
                .dn
                .split(',')
                .next()
                .and_then(|c| c.strip_prefix("DC="))
                .unwrap_or("")
                .to_string();
            if let Some(r) = e.attrs.get("dnsRecord").and_then(|v| v.first()) {
                recs.push(DnsRecord {
                    name,
                    record_type: parse_dns_type(r.as_bytes()).0,
                    value: parse_dns_type(r.as_bytes()).1,
                    ttl: 3600,
                });
            }
        }
        Ok(recs)
    }

    async fn domain_info(&self) -> Result<DomainInfo, AppError> {
        if self.mock {
            return Ok(DomainInfo {
                realm: self.settings.realm.clone(),
                workgroup: self.settings.workgroup.clone(),
                dc_host: self.settings.dc_host.clone(),
                forest_level: 2016,
                domain_level: 2016,
                users: 42,
                groups: 12,
                computers: 28,
            });
        }
        let users = self.list_users().await?;
        let groups = self.list_groups().await?;
        Ok(DomainInfo {
            realm: self.settings.realm.clone(),
            workgroup: self.settings.workgroup.clone(),
            dc_host: self.settings.dc_host.clone(),
            forest_level: 2016,
            domain_level: 2016,
            users: users.len() as u32,
            groups: groups.len() as u32,
            computers: 0,
        })
    }

    async fn join_station(&self, hostname: &str, _mac: &str) -> Result<(), AppError> {
        if self.mock {
            tracing::info!(target: "samba", %hostname, "[mock] join station");
            return Ok(());
        }
        // Real implementation would shell out to `samba-tool domain join`.
        Err(AppError::ServiceUnavailable(
            "join_station requires samba-tool on PATH".into(),
        ))
    }

    async fn leave_station(&self, hostname: &str) -> Result<(), AppError> {
        if self.mock {
            tracing::info!(target: "samba", %hostname, "[mock] leave station");
            return Ok(());
        }
        Err(AppError::ServiceUnavailable(
            "leave_station requires samba-tool on PATH".into(),
        ))
    }
}

fn parse_dns_type(rec: &[u8]) -> (String, String) {
    // RFC 1035 zone serialisation is complex; we extract a printable form
    // best-effort.
    let s = String::from_utf8_lossy(rec).to_string();
    let typ = if s.contains("A ") {
        "A"
    } else if s.contains("AAAA ") {
        "AAAA"
    } else if s.contains("CNAME ") {
        "CNAME"
    } else if s.contains("SRV ") {
        "SRV"
    } else if s.contains("SOA ") {
        "SOA"
    } else {
        "UNKNOWN"
    };
    (typ.into(), s.trim().chars().take(255).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_returns_admin() {
        let s = SambaIntegration::new(SambaSettings::default(), true);
        let users = s.list_users().await.unwrap();
        assert!(users.iter().any(|u| u.username == "Administrator"));
        let admin = s.get_user("Administrator").await.unwrap().unwrap();
        assert!(!admin.disabled);
    }

    #[tokio::test]
    async fn mock_create_and_get() {
        let s = SambaIntegration::new(SambaSettings::default(), true);
        s.create_user("bob", "Hunter22x!", Some("Bob"))
            .await
            .unwrap();
        let bob = s.get_user("bob").await.unwrap().unwrap();
        assert_eq!(bob.display_name.as_deref(), Some("Bob"));
    }
}
