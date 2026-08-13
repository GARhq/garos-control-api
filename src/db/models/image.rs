use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ImageRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nixos_version: Option<String>,
    pub kernel: Option<String>,
    pub kernel_args: Option<String>,
    pub size_mb: Option<i64>,
    pub status: String,
    pub packages_json: Option<String>,
    pub custom_nix: Option<String>,
    pub author_id: Option<String>,
    pub version: String,
    pub parent_id: Option<String>,
    pub build_log: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ImageVersionRow {
    pub id: String,
    pub image_id: String,
    pub version: String,
    pub size_mb: Option<i64>,
    pub packages_json: Option<String>,
    pub custom_nix: Option<String>,
    pub change_summary: Option<String>,
    pub author_id: Option<String>,
    pub created_at: DateTime<Utc>,
}
