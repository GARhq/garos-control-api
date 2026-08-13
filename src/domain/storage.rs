use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StoragePool {
    pub name: String,
    pub path: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_pct: f32,
    pub subvolume_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScrubStatus {
    pub pool: String,
    pub running: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub errors_found: u32,
    pub bytes_scanned: u64,
    pub progress_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Snapshot {
    pub id: Uuid,
    pub pool: String,
    pub subvolume: String,
    pub name: String,
    pub size_bytes: i64,
    pub read_only: bool,
    pub retention_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct SnapshotCreate {
    #[validate(length(min = 1, max = 128))]
    pub subvolume: String,
    pub name: Option<String>,
    pub read_only: Option<bool>,
    pub retention_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Drive {
    pub path: String,
    pub model: String,
    pub serial: String,
    pub health: String,
    pub temperature_c: Option<f64>,
    pub power_on_hours: Option<u64>,
    pub size_bytes: u64,
    pub rotation_rpm: Option<u32>,
    pub is_ssd: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NfsExport {
    pub id: Uuid,
    pub path: String,
    pub allowed_clients: String,
    pub options: String,
    pub writable: bool,
    pub sync: bool,
    pub enabled: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct NfsExportSpec {
    #[validate(length(min = 1, max = 1024))]
    pub path: String,
    pub allowed_clients: String,
    pub options: Option<String>,
    pub writable: Option<bool>,
    pub sync: Option<bool>,
    pub description: Option<String>,
}
