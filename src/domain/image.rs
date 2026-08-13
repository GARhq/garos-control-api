use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImageView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub nixos_version: Option<String>,
    pub kernel: Option<String>,
    pub kernel_args: Option<String>,
    pub size_mb: Option<i64>,
    pub status: String,
    pub version: String,
    pub parent_id: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct ImageCreate {
    #[validate(length(min = 1, max = 128))]
    pub name: String,
    pub description: Option<String>,
    pub nixos_version: Option<String>,
    pub kernel: Option<String>,
    pub kernel_args: Option<String>,
    pub packages: Vec<String>,
    pub custom_nix: Option<String>,
    #[validate(length(min = 1, max = 32))]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct ImageUpdate {
    pub description: Option<String>,
    pub kernel_args: Option<String>,
    pub packages: Option<Vec<String>>,
    pub custom_nix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImageBuildStatus {
    pub image_id: Uuid,
    pub status: String,
    pub progress_pct: f32,
    pub current_step: Option<String>,
    pub log_tail: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImageVersion {
    pub id: Uuid,
    pub image_id: Uuid,
    pub version: String,
    pub size_mb: Option<i64>,
    pub change_summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImageDiff {
    pub image_id: Uuid,
    pub version_a: String,
    pub version_b: String,
    pub packages_added: Vec<String>,
    pub packages_removed: Vec<String>,
    pub nix_diff: Option<String>,
}
