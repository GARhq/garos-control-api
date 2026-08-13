//! Storage business logic.

use crate::db::models::storage_snapshot::{NfsExportRow, StorageSnapshotRow};
use crate::db::repositories::audit::AuditRepo;
use crate::db::repositories::storage::StorageRepo;
use crate::domain::storage::*;
use crate::error::AppError;
use crate::integrations::btrfs::BtrfsIntegration;
use std::sync::Arc;
use uuid::Uuid;

pub struct StorageService {
    repo: StorageRepo,
    audit: AuditRepo,
    btrfs: Arc<BtrfsIntegration>,
}

impl StorageService {
    pub fn new(repo: StorageRepo, audit: AuditRepo, btrfs: Arc<BtrfsIntegration>) -> Self {
        Self {
            repo,
            audit,
            btrfs,
        }
    }

    pub fn repo(&self) -> &StorageRepo {
        &self.repo
    }

    pub async fn pools(&self) -> Result<Vec<StoragePool>, AppError> {
        self.btrfs.pools().await
    }

    pub async fn usage(&self, name: &str) -> Result<StoragePool, AppError> {
        self.btrfs.usage(name).await
    }

    pub async fn start_scrub(&self, pool: &str) -> Result<ScrubStatus, AppError> {
        self.btrfs.start_scrub(pool).await
    }

    pub async fn scrub_status(&self, pool: &str) -> Result<ScrubStatus, AppError> {
        self.btrfs.scrub_status(pool).await
    }

    pub async fn snapshots(&self) -> Result<Vec<Snapshot>, AppError> {
        let rows = self.repo.list_snapshots().await?;
        Ok(rows.into_iter().map(snap_from_row).collect())
    }

    pub async fn create_snapshot(
        &self,
        req: SnapshotCreate,
        actor: &str,
    ) -> Result<Snapshot, AppError> {
        req.validate()?;
        let name = req
            .name
            .clone()
            .unwrap_or_else(|| format!("snap-{}", Uuid::now_v7()));
        let snap = self
            .btrfs
            .create_snapshot(&req.subvolume, Some(&name), req.read_only.unwrap_or(true))
            .await?;
        let row = self
            .repo
            .create_snapshot(
                "garos",
                &snap.subvolume,
                &snap.name,
                snap.size_bytes,
                snap.read_only,
                req.retention_until,
                Some(actor),
            )
            .await?;
        self.audit
            .record(
                None,
                Some(actor),
                "storage.snapshot.create",
                Some("snapshot"),
                Some(&row.id),
                None,
                Some(&serde_json::to_value(snap_from_row(row.clone())).unwrap_or_default()),
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(snap_from_row(row))
    }

    pub async fn restore_snapshot(
        &self,
        id: &Uuid,
        target: &str,
        actor: &str,
    ) -> Result<(), AppError> {
        self.btrfs.restore_snapshot(id, target).await?;
        self.audit
            .record(
                None,
                Some(actor),
                "storage.snapshot.restore",
                Some("snapshot"),
                Some(&id.to_string()),
                None,
                Some(&serde_json::json!({ "target": target })),
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn delete_snapshot(&self, id: &Uuid, actor: &str) -> Result<(), AppError> {
        self.btrfs.delete_snapshot(id).await?;
        self.repo.delete_snapshot(id).await?;
        self.audit
            .record(
                None,
                Some(actor),
                "storage.snapshot.delete",
                Some("snapshot"),
                Some(&id.to_string()),
                None,
                None,
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn drives(&self) -> Result<Vec<Drive>, AppError> {
        self.btrfs.drives().await
    }

    pub async fn exports(&self) -> Result<Vec<NfsExport>, AppError> {
        let rows = self.repo.list_exports().await?;
        Ok(rows.into_iter().map(export_from_row).collect())
    }

    pub async fn create_export(
        &self,
        req: NfsExportSpec,
        actor: &str,
    ) -> Result<NfsExport, AppError> {
        req.validate()?;
        let opts = req
            .options
            .unwrap_or_else(|| "ro,sync,no_subtree_check".into());
        let row = self
            .repo
            .create_export(
                &req.path,
                &req.allowed_clients,
                &opts,
                req.writable.unwrap_or(false),
                req.sync.unwrap_or(true),
                req.description.as_deref(),
            )
            .await?;
        self.audit
            .record(
                None,
                Some(actor),
                "storage.export.create",
                Some("nfs_export"),
                Some(&row.id),
                None,
                Some(&serde_json::to_value(export_from_row(row.clone())).unwrap_or_default()),
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(export_from_row(row))
    }

    pub async fn delete_export(&self, path: &str, actor: &str) -> Result<(), AppError> {
        self.repo.delete_export(path).await?;
        self.audit
            .record(
                None,
                Some(actor),
                "storage.export.delete",
                Some("nfs_export"),
                Some(path),
                None,
                None,
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(())
    }
}

pub fn snap_from_row(row: StorageSnapshotRow) -> Snapshot {
    Snapshot {
        id: Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil),
        pool: row.pool,
        subvolume: row.subvolume,
        name: row.name,
        size_bytes: row.size_bytes,
        read_only: row.read_only,
        retention_until: row.retention_until,
        created_at: row.created_at,
    }
}

pub fn export_from_row(row: NfsExportRow) -> NfsExport {
    NfsExport {
        id: Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil),
        path: row.path,
        allowed_clients: row.allowed_clients,
        options: row.options,
        writable: row.writable,
        sync: row.sync,
        enabled: row.enabled,
        description: row.description,
    }
}
