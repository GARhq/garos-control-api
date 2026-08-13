use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StorageSnapshotRow {
    pub id: String,
    pub pool: String,
    pub subvolume: String,
    pub name: String,
    pub size_bytes: i64,
    pub read_only: bool,
    pub retention_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NfsExportRow {
    pub id: String,
    pub path: String,
    pub allowed_clients: String,
    pub options: String,
    pub writable: bool,
    pub sync: bool,
    pub enabled: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
