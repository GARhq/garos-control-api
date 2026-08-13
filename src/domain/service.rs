use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ServiceView {
    pub name: String,
    pub description: Option<String>,
    pub state: String,
    pub sub_state: Option<String>,
    pub active_for_secs: Option<u64>,
    pub main_pid: Option<u32>,
    pub memory_bytes: Option<u64>,
    pub cpu_usage_pct: Option<f64>,
    pub unit_file_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ServiceHealth {
    pub name: String,
    pub healthy: bool,
    pub consecutive_failures: i32,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub needs_attention: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub priority: i32,
    pub unit: String,
    pub message: String,
}
