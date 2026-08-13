use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FirewallRuleView {
    pub id: Uuid,
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
    pub priority: i32,
    pub nft_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct FirewallRuleCreate {
    #[validate(length(min = 1, max = 16))]
    pub action: String,
    #[validate(length(min = 1, max = 16))]
    pub family: Option<String>,
    #[validate(length(min = 1, max = 32))]
    pub chain: Option<String>,
    pub protocol: Option<String>,
    #[validate(range(min = 0, max = 65535))]
    pub port: Option<i32>,
    #[validate(range(min = 0, max = 65535))]
    pub port_end: Option<i32>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub interface_in: Option<String>,
    pub interface_out: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct FirewallRuleUpdate {
    pub action: Option<String>,
    pub protocol: Option<String>,
    pub port: Option<i32>,
    pub port_end: Option<i32>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FirewallRulePreview {
    pub command: String,
    pub warnings: Vec<String>,
    pub conflict_with: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConnectionEntry {
    pub protocol: String,
    pub source: String,
    pub destination: String,
    pub state: String,
    pub age_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PanicStatus {
    pub active: bool,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub activated_by: Option<String>,
}
