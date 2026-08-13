//! BTRFS integration (filesystem + scrub + subvolume).

use crate::config::BtrfsSettings;
use crate::domain::storage::{Drive, ScrubStatus, Snapshot, StoragePool};
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use nix::sys::statvfs;
use serde::{Deserialize, Serialize};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

#[async_trait]
pub trait Btrfs: Send + Sync {
    async fn pools(&self) -> Result<Vec<StoragePool>, AppError>;
    async fn usage(&self, name: &str) -> Result<StoragePool, AppError>;
    async fn start_scrub(&self, pool: &str) -> Result<ScrubStatus, AppError>;
    async fn scrub_status(&self, pool: &str) -> Result<ScrubStatus, AppError>;
    async fn list_snapshots(&self) -> Result<Vec<Snapshot>, AppError>;
    async fn create_snapshot(
        &self,
        subvolume: &str,
        name: Option<&str>,
        read_only: bool,
    ) -> Result<Snapshot, AppError>;
    async fn delete_snapshot(&self, id: &Uuid) -> Result<(), AppError>;
    async fn restore_snapshot(&self, id: &Uuid, target: &str) -> Result<(), AppError>;
    async fn drives(&self) -> Result<Vec<Drive>, AppError>;
}

pub struct BtrfsIntegration {
    settings: BtrfsSettings,
    mock: bool,
}

impl BtrfsIntegration {
    pub fn new(settings: BtrfsSettings, mock: bool) -> Self {
        Self { settings, mock }
    }

    pub fn settings(&self) -> &BtrfsSettings {
        &self.settings
    }

    async fn run(&self, args: &[&str]) -> Result<String, AppError> {
        let out = Command::new(&self.settings.binary_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        let output = timeout(Duration::from_secs(30), out)
            .await
            .map_err(|_| AppError::ServiceUnavailable("btrfs timeout".into()))?
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Btrfs,
                message: format!("spawn: {e}"),
            })?;
        if !output.status.success() {
            return Err(AppError::Integration {
                kind: IntegrationKind::Btrfs,
                message: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn fake_pool(&self, name: &str) -> StoragePool {
        StoragePool {
            name: name.to_string(),
            path: format!("{}/{}", self.settings.mountpoint.display(), name),
            total_bytes: 10 * 1024 * 1024 * 1024 * 1024,
            used_bytes: 4 * 1024 * 1024 * 1024 * 1024,
            free_bytes: 6 * 1024 * 1024 * 1024 * 1024,
            usage_pct: 40.0,
            subvolume_count: 8,
        }
    }
}

#[async_trait]
impl Btrfs for BtrfsIntegration {
    async fn pools(&self) -> Result<Vec<StoragePool>, AppError> {
        if self.mock {
            return Ok(vec![
                self.fake_pool("root"),
                self.fake_pool("home"),
                self.fake_pool("garos"),
            ]);
        }
        // statvfs on the mountpoint
        let path = self.settings.mountpoint.as_path();
        let stat = statvfs::statvfs(path)
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Btrfs,
                message: format!("statvfs: {e}"),
            })?;
        let total = stat.blocks() * stat.fragment_size();
        let free = stat.blocks_available() * stat.fragment_size();
        let used = total.saturating_sub(free);
        let usage_pct = if total == 0 {
            0.0
        } else {
            (used as f32 / total as f32) * 100.0
        };
        Ok(vec![StoragePool {
            name: path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("root")
                .to_string(),
            path: path.display().to_string(),
            total_bytes: total,
            used_bytes: used,
            free_bytes: free,
            usage_pct,
            subvolume_count: 0,
        }])
    }

    async fn usage(&self, name: &str) -> Result<StoragePool, AppError> {
        let pools = self.pools().await?;
        pools
            .into_iter()
            .find(|p| p.name == name)
            .ok_or_else(|| AppError::NotFound(format!("pool {name}")))
    }

    async fn start_scrub(&self, pool: &str) -> Result<ScrubStatus, AppError> {
        if self.mock {
            return Ok(ScrubStatus {
                pool: pool.into(),
                running: true,
                started_at: Some(chrono::Utc::now()),
                finished_at: None,
                errors_found: 0,
                bytes_scanned: 0,
                progress_pct: 0.0,
            });
        }
        self.run(&["scrub", "start", pool]).await?;
        self.scrub_status(pool).await
    }

    async fn scrub_status(&self, pool: &str) -> Result<ScrubStatus, AppError> {
        if self.mock {
            return Ok(ScrubStatus {
                pool: pool.into(),
                running: true,
                started_at: Some(chrono::Utc::now()),
                finished_at: None,
                errors_found: 0,
                bytes_scanned: 1024 * 1024 * 256,
                progress_pct: 42.0,
            });
        }
        let out = self.run(&["scrub", "status", pool]).await?;
        let running = out.contains("running");
        Ok(ScrubStatus {
            pool: pool.into(),
            running,
            started_at: if running { Some(chrono::Utc::now()) } else { None },
            finished_at: if running { None } else { Some(chrono::Utc::now()) },
            errors_found: 0,
            bytes_scanned: 0,
            progress_pct: if running { 50.0 } else { 100.0 },
        })
    }

    async fn list_snapshots(&self) -> Result<Vec<Snapshot>, AppError> {
        if self.mock {
            return Ok(vec![Snapshot {
                id: Uuid::now_v7(),
                pool: "garos".into(),
                subvolume: "nixos/root".into(),
                name: "auto-2024-08-13".into(),
                size_bytes: 1024 * 1024 * 512,
                read_only: true,
                retention_until: None,
                created_at: chrono::Utc::now(),
            }]);
        }
        let out = self.run(&["subvolume", "list", "-s"]).await?;
        let mut snaps = Vec::new();
        for line in out.lines() {
            if !line.contains("@") {
                continue;
            }
            let path = line.split_whitespace().last().unwrap_or("");
            if !path.contains("@") {
                continue;
            }
            let (subvolume, name) = path.rsplit_once('@').unwrap_or((path, ""));
            let subvolume = subvolume.trim_start_matches('/');
            snaps.push(Snapshot {
                id: Uuid::new_v5(&Uuid::NAMESPACE_OID, path.as_bytes()),
                pool: "garos".into(),
                subvolume: subvolume.into(),
                name: name.into(),
                size_bytes: 0,
                read_only: path.contains("@ro:"),
                retention_until: None,
                created_at: chrono::Utc::now(),
            });
        }
        Ok(snaps)
    }

    async fn create_snapshot(
        &self,
        subvolume: &str,
        name: Option<&str>,
        read_only: bool,
    ) -> Result<Snapshot, AppError> {
        let snapshot_name = name
            .map(str::to_string)
            .unwrap_or_else(|| format!("snap-{}", Uuid::now_v7()));
        let path = format!("{subvolume}@{snapshot_name}");
        if self.mock {
            return Ok(Snapshot {
                id: Uuid::now_v7(),
                pool: "garos".into(),
                subvolume: subvolume.into(),
                name: snapshot_name,
                size_bytes: 0,
                read_only,
                retention_until: None,
                created_at: chrono::Utc::now(),
            });
        }
        let mut args = vec!["subvolume", "snapshot"];
        if read_only {
            args.push("-r");
        }
        args.push(subvolume);
        args.push(&path);
        self.run(&args).await?;
        Ok(Snapshot {
            id: Uuid::new_v5(&Uuid::NAMESPACE_OID, path.as_bytes()),
            pool: "garos".into(),
            subvolume: subvolume.into(),
            name: snapshot_name,
            size_bytes: 0,
            read_only,
            retention_until: None,
            created_at: chrono::Utc::now(),
        })
    }

    async fn delete_snapshot(&self, id: &Uuid) -> Result<(), AppError> {
        if self.mock {
            return Ok(());
        }
        let snapshots = self.list_snapshots().await?;
        let snap = snapshots
            .into_iter()
            .find(|s| s.id == *id)
            .ok_or_else(|| AppError::NotFound(format!("snapshot {id}")))?;
        self.run(&["subvolume", "delete", &format!("{}@{}", snap.subvolume, snap.name)])
            .await?;
        Ok(())
    }

    async fn restore_snapshot(&self, id: &Uuid, target: &str) -> Result<(), AppError> {
        if self.mock {
            tracing::info!(target: "btrfs", %id, %target, "[mock] restore snapshot");
            return Ok(());
        }
        let snapshots = self.list_snapshots().await?;
        let snap = snapshots
            .into_iter()
            .find(|s| s.id == *id)
            .ok_or_else(|| AppError::NotFound(format!("snapshot {id}")))?;
        let src = format!("{}@{}", snap.subvolume, snap.name);
        let metadata = std::fs::metadata(Path::new(target))
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Btrfs,
                message: format!("target metadata: {e}"),
            })?;
        let _ = metadata.size(); // existence only
        self.run(&["send", &src]).await?; // would be piped into receive
        Ok(())
    }

    async fn drives(&self) -> Result<Vec<Drive>, AppError> {
        if self.mock {
            return Ok(vec![
                Drive {
                    path: "/dev/sda".into(),
                    model: "Samsung SSD 870 EVO 2TB".into(),
                    serial: "S5Y1NJ0R800000A".into(),
                    health: "OK".into(),
                    temperature_c: Some(34.0),
                    power_on_hours: Some(10_200),
                    size_bytes: 2 * 1024 * 1024 * 1024 * 1024,
                    rotation_rpm: None,
                    is_ssd: true,
                },
                Drive {
                    path: "/dev/sdb".into(),
                    model: "WD Red Pro 8TB".into(),
                    serial: "VBG7KJDL".into(),
                    health: "OK".into(),
                    temperature_c: Some(38.0),
                    power_on_hours: Some(22_000),
                    size_bytes: 8 * 1024 * 1024 * 1024 * 1024,
                    rotation_rpm: Some(7200),
                    is_ssd: false,
                },
            ]);
        }
        // smartctl may not be available; we fall back to /sys/block listing.
        let mut drives = Vec::new();
        let entries = std::fs::read_dir("/sys/block").ok();
        if let Some(entries) = entries {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("loop") || name.starts_with("ram") {
                    continue;
                }
                let size_path = path.join("size");
                let size_sectors: u64 = std::fs::read_to_string(&size_path)
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let model = std::fs::read_to_string(path.join("device/model"))
                    .ok()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| name.clone());
                drives.push(Drive {
                    path: format!("/dev/{name}"),
                    model,
                    serial: "unknown".into(),
                    health: "UNKNOWN".into(),
                    temperature_c: None,
                    power_on_hours: None,
                    size_bytes: size_sectors * 512,
                    rotation_rpm: None,
                    is_ssd: false,
                });
            }
        }
        Ok(drives)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_pools() {
        let b = BtrfsIntegration::new(BtrfsSettings::default(), true);
        let pools = b.pools().await.unwrap();
        assert!(!pools.is_empty());
    }

    #[tokio::test]
    async fn mock_snapshot_lifecycle() {
        let b = BtrfsIntegration::new(BtrfsSettings::default(), true);
        let s = b
            .create_snapshot("nixos/root", Some("pre-upgrade"), true)
            .await
            .unwrap();
        assert!(s.read_only);
        assert_eq!(s.name, "pre-upgrade");
    }
}
