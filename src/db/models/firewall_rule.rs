use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FirewallRuleRow {
    pub id: String,
    pub action: String,
    pub family: String,
    pub table_name: String,
    pub chain: String,
    pub protocol: Option<String>,
    pub port: Option<i32>,
    pub port_end: Option<i32>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub interface_in: Option<String>,
    pub interface_out: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub nft_handle: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
}
