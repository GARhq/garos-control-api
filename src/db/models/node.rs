use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeRow {
    pub id: String,
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
    pub serial: Option<String>,
    pub location: Option<String>,
    pub current_user_id: Option<String>,
    pub current_user_role: Option<String>,
    pub login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
