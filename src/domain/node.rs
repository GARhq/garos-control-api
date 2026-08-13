use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetbootDevice {
    pub id: Uuid,
    pub mac: String,
    pub hostname: Option<String>,
    pub ip: Option<String>,
    pub status: String,
    pub image_id: Option<String>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub cpu_temp_c: Option<f64>,
    pub cpu_usage_pct: Option<f64>,
    pub mem_usage_pct: Option<f64>,
    pub fan_rpm: Option<i32>,
    pub ping_ms: Option<f64>,
    pub nfs_latency_ms: Option<f64>,
    pub hardware_model: Option<String>,
    pub current_user_id: Option<String>,
    pub current_user_role: Option<String>,
    pub login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct MacAddress(pub String);

impl std::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl MacAddress {
    pub fn is_valid(s: &str) -> bool {
        // Accept AA:BB:CC:DD:EE:FF or AA-BB-CC-DD-EE-FF
        let parts: Vec<&str> = s.split([':', '-']).collect();
        if parts.len() != 6 {
            return false;
        }
        parts
            .iter()
            .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct HeartbeatRequest {
    pub hostname: Option<String>,
    pub ip: Option<String>,
    pub cpu_temp_c: Option<f64>,
    pub cpu_usage_pct: Option<f64>,
    pub mem_usage_pct: Option<f64>,
    pub ping_ms: Option<f64>,
    pub nfs_latency_ms: Option<f64>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct ReimageRequest {
    pub image_id: Uuid,
    #[validate(length(max = 256))]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct BulkMacRequest {
    #[validate(length(min = 1, max = 4096))]
    pub macs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct BulkReimageRequest {
    pub macs: Vec<String>,
    pub image_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NodeStats {
    pub total: i64,
    pub by_status: std::collections::HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WolResult {
    pub mac: String,
    pub sent_at: DateTime<Utc>,
    pub broadcast: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkActionResult {
    pub accepted: usize,
    pub rejected: usize,
    pub details: Vec<BulkActionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkActionItem {
    pub mac: String,
    pub ok: bool,
    pub error: Option<String>,
}
