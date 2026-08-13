//! Storage repositories.

use crate::db::models::storage_snapshot::{NfsExportRow, StorageSnapshotRow};
use crate::db::pool::DbPool;
use crate::error::AppError;
use uuid::Uuid;

#[derive(Clone)]
pub struct StorageRepo {
    pool: DbPool,
}

impl StorageRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_snapshots(&self) -> Result<Vec<StorageSnapshotRow>, AppError> {
        let rows = sqlx::query_as::<_, StorageSnapshotRow>(
            "SELECT * FROM storage_snapshots ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn snapshot_by_id(
        &self,
        id: &Uuid,
    ) -> Result<Option<StorageSnapshotRow>, AppError> {
        let row = sqlx::query_as::<_, StorageSnapshotRow>(
            "SELECT * FROM storage_snapshots WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn create_snapshot(
        &self,
        pool: &str,
        subvolume: &str,
        name: &str,
        size_bytes: i64,
        read_only: bool,
        retention_until: Option<chrono::DateTime<chrono::Utc>>,
        created_by: Option<&str>,
    ) -> Result<StorageSnapshotRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO storage_snapshots (id, pool, subvolume, name, size_bytes, read_only, retention_until, created_at, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(pool)
        .bind(subvolume)
        .bind(name)
        .bind(size_bytes)
        .bind(read_only)
        .bind(retention_until)
        .bind(now)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(StorageSnapshotRow {
            id,
            pool: pool.to_string(),
            subvolume: subvolume.to_string(),
            name: name.to_string(),
            size_bytes,
            read_only,
            retention_until,
            created_at: now,
            created_by: created_by.map(str::to_string),
        })
    }

    pub async fn delete_snapshot(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM storage_snapshots WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- NFS exports ----

    pub async fn list_exports(&self) -> Result<Vec<NfsExportRow>, AppError> {
        let rows = sqlx::query_as::<_, NfsExportRow>("SELECT * FROM nfs_exports ORDER BY path ASC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn create_export(
        &self,
        path: &str,
        allowed_clients: &str,
        options: &str,
        writable: bool,
        sync: bool,
        description: Option<&str>,
    ) -> Result<NfsExportRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO nfs_exports (id, path, allowed_clients, options, writable, sync, enabled, description, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, ?)",
        )
        .bind(&id)
        .bind(path)
        .bind(allowed_clients)
        .bind(options)
        .bind(writable)
        .bind(sync)
        .bind(description)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(NfsExportRow {
            id,
            path: path.to_string(),
            allowed_clients: allowed_clients.to_string(),
            options: options.to_string(),
            writable,
            sync,
            enabled: true,
            description: description.map(str::to_string),
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn delete_export(&self, path: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM nfs_exports WHERE path = ?")
            .bind(path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
