use crate::auth::jwt::Role;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub password_hash: Option<String>,
    pub role: String,
    pub status: String,
    pub quota_used_bytes: i64,
    pub quota_limit_bytes: Option<i64>,
    pub failed_login_count: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub force_password_change: bool,
    pub samba_dn: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
}

impl UserRow {
    pub fn id(&self) -> Uuid {
        Uuid::parse_str(&self.id).unwrap_or_else(|_| Uuid::now_v7())
    }

    pub fn role(&self) -> Role {
        Role::from_str(&self.role).unwrap_or(Role::User)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActiveSessionRow {
    pub id: String,
    pub user_id: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub login_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
