//! journald reader: queries by hostname, priority, since, until, unit.

use crate::config::JournaldSettings;
use crate::domain::service::LogLine;
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

#[async_trait]
pub trait Journald: Send + Sync {
    async fn query(
        &self,
        unit: Option<&str>,
        hostname: Option<&str>,
        priority: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        max_bytes: u64,
    ) -> Result<Vec<LogLine>, AppError>;
    async fn stream(
        &self,
        unit: Option<&str>,
    ) -> Result<tokio::sync::mpsc::Receiver<LogLine>, AppError>;
}

pub struct JournaldIntegration {
    settings: JournaldSettings,
    mock: bool,
}

impl JournaldIntegration {
    pub fn new(settings: JournaldSettings, mock: bool) -> Self {
        Self { settings, mock }
    }
}

#[async_trait]
impl Journald for JournaldIntegration {
    async fn query(
        &self,
        unit: Option<&str>,
        hostname: Option<&str>,
        priority: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        max_bytes: u64,
    ) -> Result<Vec<LogLine>, AppError> {
        if self.mock {
            let now = chrono::Utc::now();
            return Ok((0..50)
                .map(|i| LogLine {
                    timestamp: now - chrono::Duration::seconds(i as i64),
                    priority: 6,
                    unit: unit.unwrap_or("garos-backend").into(),
                    message: format!("[mock] journal line {i}"),
                })
                .collect());
        }
        let mut args: Vec<String> = vec![
            "-n".into(),
            "200".into(),
            "-o".into(),
            "short".into(),
            "--no-pager".into(),
        ];
        if let Some(u) = unit {
            args.push("-u".into());
            args.push(u.into());
        }
        if let Some(h) = hostname {
            args.push("--host".into());
            args.push(h.into());
        }
        if let Some(p) = priority {
            args.push("-p".into());
            args.push(p.into());
        }
        if let Some(s) = since {
            args.push("-S".into());
            args.push(s.into());
        }
        if let Some(u) = until {
            args.push("-U".into());
            args.push(u.into());
        }
        let max = self.settings.max_bytes.min(max_bytes);
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let child = Command::new("journalctl")
            .args(&argv)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let child = match child {
            Ok(c) => c,
            Err(e) => {
                return Err(AppError::Integration {
                    kind: IntegrationKind::Journald,
                    message: format!("spawn: {e}"),
                })
            }
        };
        let out = timeout(self.settings.timeout(), child.wait_with_output())
            .await
            .map_err(|_| AppError::ServiceUnavailable("journalctl timeout".into()))?
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Journald,
                message: format!("wait: {e}"),
            })?;
        let text = String::from_utf8_lossy(&out.stdout);
        if text.len() as u64 > max {
            return Err(AppError::Integration {
                kind: IntegrationKind::Journald,
                message: format!("response exceeded {} bytes", max),
            });
        }
        let mut logs = Vec::new();
        for line in text.lines() {
            if let Some(parsed) = parse_line(line, unit.unwrap_or("garos-backend")) {
                logs.push(parsed);
            }
        }
        Ok(logs)
    }

    async fn stream(
        &self,
        unit: Option<&str>,
    ) -> Result<tokio::sync::mpsc::Receiver<LogLine>, AppError> {
        let (tx, rx) = tokio::sync::mpsc::channel::<LogLine>(256);
        if self.mock {
            tokio::spawn(async move {
                let mut i = 0u32;
                loop {
                    let line = LogLine {
                        timestamp: chrono::Utc::now(),
                        priority: 6,
                        unit: unit.unwrap_or("garos-backend").into(),
                        message: format!("[mock stream] {i}"),
                    };
                    if tx.send(line).await.is_err() {
                        break;
                    }
                    i += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            });
            return Ok(rx);
        }
        let mut args: Vec<String> = vec!["-f".into(), "-o".into(), "short".into()];
        if let Some(u) = unit {
            args.push("-u".into());
            args.push(u.into());
        }
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut child = Command::new("journalctl")
            .args(&argv)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Journald,
                message: format!("spawn: {e}"),
            })?;
        let stdout = child.stdout.take().ok_or_else(|| AppError::Integration {
            kind: IntegrationKind::Journald,
            message: "no stdout".into(),
        })?;
        let unit_owned = unit.unwrap_or("garos-backend").to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(parsed) = parse_line(&line, &unit_owned) {
                    if tx.send(parsed).await.is_err() {
                        break;
                    }
                }
            }
        });
        Ok(rx)
    }
}

fn parse_line(line: &str, unit: &str) -> Option<LogLine> {
    let mut parts = line.splitn(5, ' ');
    let _month = parts.next()?;
    let _day = parts.next()?;
    let _time = parts.next()?;
    let _host = parts.next()?;
    let rest = parts.next()?;
    let (unit_part, msg) = rest.split_once(':')?;
    let u = unit_part.split('[').next().unwrap_or(unit).trim();
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
        unit: u.to_string(),
        message: msg.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_journal_line_basic() {
        let line = "Aug 13 12:34:56 host garos[1]: hello";
        let l = parse_line(line, "garos").unwrap();
        assert_eq!(l.unit, "garos");
    }
}
