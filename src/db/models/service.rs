use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceHealthStateRow {
    pub service_name: String,
    pub consecutive_failures: i32,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub needs_attention: bool,
    pub last_status_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IdempotencyKeyRow {
    pub key: String,
    pub user_id: String,
    pub method: String,
    pub path: String,
    pub request_hash: String,
    pub status: i32,
    pub response_json: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
