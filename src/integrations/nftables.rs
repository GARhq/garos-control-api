//! NFTables integration: list, add, delete, panic, validate.

use crate::config::NftablesSettings;
use crate::db::models::firewall_rule::FirewallRuleRow;
use crate::domain::firewall::{ConnectionEntry, FirewallRulePreview, PanicStatus};
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

#[async_trait]
pub trait Nftables: Send + Sync {
    async fn list_ruleset(&self) -> Result<String, AppError>;
    async fn add_rule(&self, rule: &FirewallRuleRow) -> Result<String, AppError>;
    async fn delete_rule(&self, handle: &str) -> Result<(), AppError>;
    async fn flush_table(&self) -> Result<(), AppError>;
    async fn apply_ruleset(&self, ruleset: &str) -> Result<(), AppError>;
    async fn preview_rule(&self, rule: &FirewallRuleRow) -> Result<FirewallRulePreview, AppError>;
    async fn panic(&self, activate: bool, actor: Option<&str>) -> Result<PanicStatus, AppError>;
    async fn panic_status(&self) -> Result<PanicStatus, AppError>;
    async fn list_connections(&self, limit: usize) -> Result<Vec<ConnectionEntry>, AppError>;
    async fn validate(&self, rules: &[FirewallRuleRow]) -> Result<Vec<String>, AppError>;
}

pub struct NftablesIntegration {
    settings: NftablesSettings,
    mock: bool,
}

impl NftablesIntegration {
    pub fn new(settings: NftablesSettings, mock: bool) -> Self {
        Self { settings, mock }
    }

    async fn run(&self, args: &[&str]) -> Result<String, AppError> {
        let out = Command::new(&self.settings.binary_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        let output = timeout(Duration::from_secs(10), out)
            .await
            .map_err(|_| AppError::ServiceUnavailable("nft timeout".into()))?
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Nftables,
                message: format!("spawn: {e}"),
            })?;
        if !output.status.success() {
            return Err(AppError::Integration {
                kind: IntegrationKind::Nftables,
                message: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Build a nft command line for a rule.
    pub fn build_command(&self, rule: &FirewallRuleRow) -> String {
        let mut parts: Vec<String> = vec![
            "nft".into(),
            "add".into(),
            "rule".into(),
            self.settings.family.clone(),
            self.settings.table_name.clone(),
            rule.chain.clone(),
        ];
        if let Some(if_in) = &rule.interface_in {
            parts.push("iif".into());
            parts.push(format!("\"{if_in}\""));
        }
        if let Some(if_out) = &rule.interface_out {
            parts.push("oif".into());
            parts.push(format!("\"{if_out}\""));
        }
        if let Some(src) = &rule.source {
            parts.push("ip".into());
            parts.push("saddr".into());
            parts.push(src.clone());
        }
        if let Some(dst) = &rule.destination {
            parts.push("ip".into());
            parts.push("daddr".into());
            parts.push(dst.clone());
        }
        if let Some(proto) = &rule.protocol {
            parts.push(proto.clone());
        }
        if let Some(port) = rule.port {
            if let Some(port_end) = rule.port_end {
                parts.push("dport".into());
                parts.push(format!("{port}-{port_end}"));
            } else {
                parts.push("dport".into());
                parts.push(port.to_string());
            }
        }
        parts.push(rule.action.clone());
        if let Some(comment) = &rule.description {
            parts.push("comment".into());
            parts.push(format!("\"{comment}\""));
        }
        parts
            .iter()
            .map(|s| {
                if s.chars().any(char::is_whitespace) {
                    format!("'{s}'")
                } else {
                    s.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[async_trait]
impl Nftables for NftablesIntegration {
    async fn list_ruleset(&self) -> Result<String, AppError> {
        if self.mock {
            return Ok("table inet garos {\n  chain input { type filter hook input priority 0; policy accept; }\n}".to_string());
        }
        self.run(&["list", "ruleset"]).await
    }

    async fn add_rule(&self, rule: &FirewallRuleRow) -> Result<String, AppError> {
        let cmd = self.build_command(rule);
        if self.mock {
            return Ok(format!("[mock] {cmd}"));
        }
        // Use bash -c to handle quoting safely.
        let out = Command::new("bash")
            .arg("-c")
            .arg(&cmd)
            .output()
            .await
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Nftables,
                message: format!("spawn bash: {e}"),
            })?;
        if !out.status.success() {
            return Err(AppError::Integration {
                kind: IntegrationKind::Nftables,
                message: String::from_utf8_lossy(&out.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    async fn delete_rule(&self, handle: &str) -> Result<(), AppError> {
        if self.mock {
            return Ok(());
        }
        self.run(&["delete", "rule", &self.settings.family, &self.settings.table_name, "handle", handle])
            .await?;
        Ok(())
    }

    async fn flush_table(&self) -> Result<(), AppError> {
        if self.mock {
            return Ok(());
        }
        self.run(&["flush", "table", &self.settings.family, &self.settings.table_name])
            .await?;
        Ok(())
    }

    async fn apply_ruleset(&self, ruleset: &str) -> Result<(), AppError> {
        if self.mock {
            return Ok(());
        }
        let mut child = Command::new(&self.settings.binary_path)
            .arg("-f")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Nftables,
                message: format!("spawn: {e}"),
            })?;
        if let Some(stdin) = child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(ruleset.as_bytes()).await.map_err(|e| {
                AppError::Integration {
                    kind: IntegrationKind::Nftables,
                    message: format!("stdin: {e}"),
                }
            })?;
        }
        let out = child.wait_with_output().await.map_err(|e| AppError::Integration {
            kind: IntegrationKind::Nftables,
            message: format!("wait: {e}"),
        })?;
        if !out.status.success() {
            return Err(AppError::Integration {
                kind: IntegrationKind::Nftables,
                message: String::from_utf8_lossy(&out.stderr).into_owned(),
            });
        }
        Ok(())
    }

    async fn preview_rule(&self, rule: &FirewallRuleRow) -> Result<FirewallRulePreview, AppError> {
        Ok(FirewallRulePreview {
            command: self.build_command(rule),
            warnings: vec![],
            conflict_with: vec![],
        })
    }

    async fn panic(&self, activate: bool, actor: Option<&str>) -> Result<PanicStatus, AppError> {
        if self.mock {
            return Ok(PanicStatus {
                active: activate,
                since: if activate { Some(chrono::Utc::now()) } else { None },
                activated_by: actor.map(str::to_string),
            });
        }
        if activate {
            self.apply_ruleset(&format!(
                "table {} {} {{\n  chain input {{ type filter hook input priority 0; policy drop; comment \"garos panic\"; }}\n  chain forward {{ type filter hook forward priority 0; policy drop; }}\n  chain output {{ type filter hook output priority 0; policy accept; }}\n}}",
                self.settings.family, self.settings.table_name,
            )).await?;
        } else {
            self.flush_table().await?;
        }
        self.panic_status().await
    }

    async fn panic_status(&self) -> Result<PanicStatus, AppError> {
        if self.mock {
            return Ok(PanicStatus {
                active: false,
                since: None,
                activated_by: None,
            });
        }
        let out = self.run(&["list", "table", &self.settings.family, &self.settings.table_name]).await?;
        let active = out.contains("comment \"garos panic\"");
        Ok(PanicStatus {
            active,
            since: if active { Some(chrono::Utc::now()) } else { None },
            activated_by: None,
        })
    }

    async fn list_connections(&self, limit: usize) -> Result<Vec<ConnectionEntry>, AppError> {
        if self.mock {
            return Ok((0..limit.min(5))
                .map(|i| ConnectionEntry {
                    protocol: "tcp".into(),
                    source: format!("10.0.0.{}:{}", 100 + i, 4000 + i),
                    destination: format!("10.0.0.1:443"),
                    state: "ESTABLISHED".into(),
                    age_secs: 60 + i as u64,
                })
                .collect());
        }
        let out = self
            .run(&["list", "set", "inet", &self.settings.table_name, "conntrack"])
            .await
            .unwrap_or_default();
        let mut entries = Vec::new();
        for line in out.lines().take(limit) {
            if line.contains('{') || line.contains('}') {
                continue;
            }
            entries.push(ConnectionEntry {
                protocol: "tcp".into(),
                source: line.to_string(),
                destination: "".into(),
                state: "ESTABLISHED".into(),
                age_secs: 0,
            });
        }
        Ok(entries)
    }

    async fn validate(&self, rules: &[FirewallRuleRow]) -> Result<Vec<String>, AppError> {
        let mut conflicts = Vec::new();
        for (i, a) in rules.iter().enumerate() {
            for b in rules.iter().skip(i + 1) {
                if a.enabled && b.enabled
                    && a.protocol.is_some()
                    && a.protocol == b.protocol
                    && a.port.is_some()
                    && a.port == b.port
                    && a.action != b.action
                {
                    conflicts.push(format!(
                        "conflict: rule {} and {} both target {}/{} with different actions",
                        a.id, b.id,
                        a.protocol.as_deref().unwrap_or(""),
                        a.port.unwrap_or(0),
                    ));
                }
            }
        }
        Ok(conflicts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::firewall_rule::FirewallRuleRow;
    use chrono::Utc;

    fn rule(port: i32, action: &str) -> FirewallRuleRow {
        FirewallRuleRow {
            id: Uuid::now_v7().to_string(),
            action: action.into(),
            family: "inet".into(),
            table_name: "garos".into(),
            chain: "input".into(),
            protocol: Some("tcp".into()),
            port: Some(port),
            port_end: None,
            source: None,
            destination: None,
            interface_in: None,
            interface_out: None,
            description: Some(format!("test rule on port {port}")),
            enabled: true,
            nft_handle: None,
            priority: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
        }
    }

    #[tokio::test]
    async fn mock_panic() {
        let n = NftablesIntegration::new(NftablesSettings::default(), true);
        let st = n.panic(true, Some("admin")).await.unwrap();
        assert!(st.active);
    }

    #[tokio::test]
    async fn preview_command() {
        let n = NftablesIntegration::new(NftablesSettings::default(), true);
        let cmd = n.build_command(&rule(443, "accept"));
        assert!(cmd.contains("tcp"));
        assert!(cmd.contains("443"));
        assert!(cmd.contains("accept"));
    }

    #[tokio::test]
    async fn detect_conflict() {
        let n = NftablesIntegration::new(NftablesSettings::default(), true);
        let rules = vec![rule(80, "accept"), rule(80, "drop")];
        let conflicts = n.validate(&rules).await.unwrap();
        assert!(!conflicts.is_empty());
    }
}
