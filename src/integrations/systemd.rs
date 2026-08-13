//! systemd integration: prefer D-Bus (zbus) when `use_dbus = true`, fallback
//! to `systemctl` shell-out.

use crate::config::SystemdSettings;
use crate::db::repositories::services::ServiceHealthRepo;
use crate::domain::service::{LogLine, ServiceHealth, ServiceView};
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::warn;

#[async_trait]
pub trait Systemd: Send + Sync {
    async fn list_units(&self) -> Result<Vec<ServiceView>, AppError>;
    async fn unit(&self, name: &str) -> Result<ServiceView, AppError>;
    async fn start(&self, name: &str) -> Result<(), AppError>;
    async fn stop(&self, name: &str) -> Result<(), AppError>;
    async fn restart(&self, name: &str) -> Result<(), AppError>;
    async fn logs(
        &self,
        name: &str,
        lines: u32,
        since: Option<&str>,
        until: Option<&str>,
        priority: Option<&str>,
    ) -> Result<Vec<LogLine>, AppError>;
    async fn health(&self, name: &str, repo: &ServiceHealthRepo) -> Result<ServiceHealth, AppError>;
}

pub struct SystemdIntegration {
    settings: SystemdSettings,
    mock: bool,
}

impl SystemdIntegration {
    pub fn new(settings: SystemdSettings, mock: bool) -> Self {
        Self { settings, mock }
    }

    async fn run(&self, args: &[&str]) -> Result<String, AppError> {
        let out = Command::new(&self.settings.systemctl_binary)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        let output = timeout(std::time::Duration::from_secs(10), out)
            .await
            .map_err(|_| AppError::ServiceUnavailable("systemctl timeout".into()))?
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Systemd,
                message: format!("spawn: {e}"),
            })?;
        if !output.status.success() && !output.stderr.is_empty() {
            return Err(AppError::Integration {
                kind: IntegrationKind::Systemd,
                message: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[async_trait]
impl Systemd for SystemdIntegration {
    async fn list_units(&self) -> Result<Vec<ServiceView>, AppError> {
        if self.mock {
            return Ok(vec![
                ServiceView {
                    name: "garos-backend.service".into(),
                    description: Some("garos backend service".into()),
                    state: "active".into(),
                    sub_state: Some("running".into()),
                    active_for_secs: Some(60),
                    main_pid: Some(1),
                    memory_bytes: Some(64 * 1024 * 1024),
                    cpu_usage_pct: Some(2.5),
                    unit_file_state: Some("enabled".into()),
                },
                ServiceView {
                    name: "nftables.service".into(),
                    description: Some("nftables firewall".into()),
                    state: "active".into(),
                    sub_state: Some("exited".into()),
                    active_for_secs: Some(1200),
                    main_pid: None,
                    memory_bytes: None,
                    cpu_usage_pct: None,
                    unit_file_state: Some("enabled".into()),
                },
                ServiceView {
                    name: "samba-ad-dc.service".into(),
                    description: Some("Samba AD Domain Controller".into()),
                    state: "active".into(),
                    sub_state: Some("running".into()),
                    active_for_secs: Some(86400),
                    main_pid: Some(1234),
                    memory_bytes: Some(256 * 1024 * 1024),
                    cpu_usage_pct: Some(0.5),
                    unit_file_state: Some("enabled".into()),
                },
            ]);
        }
        // D-Bus path: if enabled, connect; otherwise shell out.
        if self.settings.use_dbus {
            match self.list_units_dbus().await {
                Ok(units) => return Ok(units),
                Err(e) => {
                    warn!(error = %e, "D-Bus systemd list failed, falling back to systemctl");
                }
            }
        }
        let out = self
            .run(&["list-units", "--type=service", "--no-pager", "--plain", "--no-legend"])
            .await?;
        let mut units = Vec::new();
        for line in out.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 5 {
                continue;
            }
            units.push(ServiceView {
                name: cols[0].to_string(),
                description: cols.get(4).map(|s| s.to_string()),
                state: cols.get(2).unwrap_or(&"unknown").to_string(),
                sub_state: cols.get(3).map(|s| s.to_string()),
                active_for_secs: None,
                main_pid: None,
                memory_bytes: None,
                cpu_usage_pct: None,
                unit_file_state: None,
            });
        }
        Ok(units)
    }

    async fn unit(&self, name: &str) -> Result<ServiceView, AppError> {
        let units = self.list_units().await?;
        units
            .into_iter()
            .find(|u| u.name == name)
            .ok_or_else(|| AppError::NotFound(format!("service {name}")))
    }

    async fn start(&self, name: &str) -> Result<(), AppError> {
        if self.mock {
            tracing::info!(target: "systemd", %name, "[mock] start");
            return Ok(());
        }
        self.run(&["start", name]).await?;
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<(), AppError> {
        if self.mock {
            tracing::info!(target: "systemd", %name, "[mock] stop");
            return Ok(());
        }
        self.run(&["stop", name]).await?;
        Ok(())
    }

    async fn restart(&self, name: &str) -> Result<(), AppError> {
        if self.mock {
            tracing::info!(target: "systemd", %name, "[mock] restart");
            return Ok(());
        }
        self.run(&["restart", name]).await?;
        Ok(())
    }

    async fn logs(
        &self,
        name: &str,
        lines: u32,
        since: Option<&str>,
        until: Option<&str>,
        priority: Option<&str>,
    ) -> Result<Vec<LogLine>, AppError> {
        if self.mock {
            return Ok((0..lines.min(20))
                .map(|i| LogLine {
                    timestamp: chrono::Utc::now() - chrono::Duration::seconds(i as i64),
                    priority: 6,
                    unit: name.into(),
                    message: format!("[mock] log line {i} for {name}"),
                })
                .collect());
        }
        let mut args = vec!["-u", name, "-n", &lines.to_string(), "--no-pager", "-o", "short"];
        if let Some(s) = since {
            args.push("-S");
            args.push(s);
        }
        if let Some(u) = until {
            args.push("-U");
            args.push(u);
        }
        if let Some(p) = priority {
            args.push("-p");
            args.push(p);
        }
        let child = Command::new("journalctl")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Systemd,
                message: format!("journalctl: {e}"),
            })?;
        let out = child.wait_with_output().await.map_err(|e| AppError::Integration {
            kind: IntegrationKind::Systemd,
            message: format!("wait: {e}"),
        })?;
        let mut logs = Vec::new();
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if let Some(parsed) = parse_journal_line(line, name) {
                logs.push(parsed);
            }
        }
        Ok(logs)
    }

    async fn health(&self, name: &str, repo: &ServiceHealthRepo) -> Result<ServiceHealth, AppError> {
        let unit = self.unit(name).await?;
        if unit.state == "active" {
            let row = repo.record_success(name).await?;
            Ok(ServiceHealth {
                name: name.into(),
                healthy: true,
                consecutive_failures: row.consecutive_failures,
                last_failure_at: row.last_failure_at,
                last_success_at: row.last_success_at,
                needs_attention: row.needs_attention,
            })
        } else {
            let row = repo.record_failure(name).await?;
            Ok(ServiceHealth {
                name: name.into(),
                healthy: false,
                consecutive_failures: row.consecutive_failures,
                last_failure_at: row.last_failure_at,
                last_success_at: row.last_success_at,
                needs_attention: row.needs_attention,
            })
        }
    }
}

impl SystemdIntegration {
    /// D-Bus path via zbus. Lazily connected; falls back to systemctl on
    /// any D-Bus error.
    async fn list_units_dbus(&self) -> Result<Vec<ServiceView>, AppError> {
        // D-Bus connection is kept simple: we use the synchronous
        // `systemctl` shell-out instead and reserve this hook for future
        // migration to `zbus::Connection`.
        Err(AppError::ServiceUnavailable(
            "D-Bus systemd path is not yet implemented; using systemctl fallback".into(),
        ))
    }
}

fn parse_journal_line(line: &str, unit: &str) -> Option<LogLine> {
    // journalctl -o short: "Aug 13 12:34:56 host unit[pid]: message"
    let mut parts = line.splitn(5, ' ');
    let _month = parts.next()?;
    let _day = parts.next()?;
    let _time = parts.next()?;
    let _host = parts.next()?;
    let rest = parts.next()?;
    let (unit_part, msg) = rest.split_once(':')?;
    let unit = unit_part.split('[').next().unwrap_or(unit).trim();
    let prio = if msg.to_lowercase().contains("err") {
        3
    } else if msg.to_lowercase().contains("warn") {
        4
    } else {
        6
    };
    Some(LogLine {
        timestamp: chrono::Utc::now(),
        priority: prio,
        unit: unit.to_string(),
        message: msg.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_list() {
        let s = SystemdIntegration::new(SystemdSettings::default(), true);
        let units = s.list_units().await.unwrap();
        assert!(!units.is_empty());
    }

    #[test]
    fn parse_log_line() {
        let line = "Aug 13 12:34:56 host garos-backend[1234]: hello world";
        let l = parse_journal_line(line, "garos-backend").unwrap();
        assert_eq!(l.unit, "garos-backend");
        assert!(l.message.contains("hello world"));
    }
}
