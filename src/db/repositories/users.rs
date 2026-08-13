//! Users repository.

use crate::db::models::user::{ActiveSessionRow, RefreshTokenRow, UserRow};
use crate::db::pool::DbPool;
use crate::error::AppError;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRepo {
    pool: DbPool,
}

impl UserRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    pub async fn count(&self) -> Result<i64, AppError> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL")
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }

    pub async fn by_id(&self, id: &Uuid) -> Result<Option<UserRow>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT * FROM users WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn by_username(&self, username: &str) -> Result<Option<UserRow>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT * FROM users WHERE username = ? AND deleted_at IS NULL",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn by_email(&self, email: &str) -> Result<Option<UserRow>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT * FROM users WHERE email = ? AND deleted_at IS NULL",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn create(
        &self,
        username: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        password_hash: Option<&str>,
        role: &str,
        samba_dn: Option<&str>,
    ) -> Result<UserRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now();
        sqlx::query(
            r#"INSERT INTO users
               (id, username, email, display_name, password_hash, role, status,
                created_at, updated_at, samba_dn)
               VALUES (?, ?, ?, ?, ?, ?, 'active', ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(username)
        .bind(email)
        .bind(display_name)
        .bind(password_hash)
        .bind(role)
        .bind(now)
        .bind(now)
        .bind(samba_dn)
        .execute(&self.pool)
        .await?;

        self.by_id(&Uuid::parse_str(&id).unwrap())
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("user vanished after insert")))
    }

    pub async fn update_password(&self, id: &Uuid, hash: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET password_hash = ?, force_password_change = 0, updated_at = ? WHERE id = ?")
            .bind(hash)
            .bind(Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_status(&self, id: &Uuid, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_role(&self, id: &Uuid, role: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET role = ?, updated_at = ? WHERE id = ?")
            .bind(role)
            .bind(Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_quota(
        &self,
        id: &Uuid,
        used: i64,
        limit: Option<i64>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET quota_used_bytes = ?, quota_limit_bytes = ?, updated_at = ? WHERE id = ?")
            .bind(used)
            .bind(limit)
            .bind(Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn record_login_failure(&self, id: &Uuid, max: i32) -> Result<bool, AppError> {
        sqlx::query(
            "UPDATE users SET failed_login_count = failed_login_count + 1, updated_at = ? WHERE id = ?",
        )
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        let row: (i32,) = sqlx::query_as("SELECT failed_login_count FROM users WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await?;
        if row.0 >= max {
            sqlx::query("UPDATE users SET locked_until = ?, status = 'blocked' WHERE id = ?")
                .bind(Utc::now() + Duration::minutes(15))
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn reset_login_failures(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET failed_login_count = 0, locked_until = NULL WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn unlock(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET status = 'active', failed_login_count = 0, locked_until = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn soft_delete(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET deleted_at = ?, status = 'disabled', updated_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list(
        &self,
        search: Option<&str>,
        role: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserRow>, AppError> {
        let mut sql = String::from("SELECT * FROM users WHERE deleted_at IS NULL");
        let mut binds: Vec<String> = vec![];
        if let Some(s) = search {
            sql.push_str(" AND (username LIKE ? OR email LIKE ? OR display_name LIKE ?)");
            let pat = format!("%{s}%");
            binds.push(pat.clone());
            binds.push(pat.clone());
            binds.push(pat);
        }
        if let Some(r) = role {
            sql.push_str(" AND role = ?");
            binds.push(r.to_string());
        }
        if let Some(s) = status {
            sql.push_str(" AND status = ?");
            binds.push(s.to_string());
        }
        sql.push_str(" ORDER BY username ASC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, UserRow>(&sql);
        for b in &binds {
            q = q.bind(b);
        }
        q = q.bind(limit).bind(offset);
        Ok(q.fetch_all(&self.pool).await?)
    }

    pub async fn stats(&self) -> Result<serde_json::Value, AppError> {
        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL")
                .fetch_one(&self.pool)
                .await?;
        let active: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND status = 'active'",
        )
        .fetch_one(&self.pool)
        .await?;
        let blocked: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND status = 'blocked'",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(serde_json::json!({
            "total": total.0,
            "active": active.0,
            "blocked": blocked.0,
        }))
    }

    // ---- refresh tokens ----

    pub async fn store_refresh(
        &self,
        user_id: &Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<RefreshTokenRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id.to_string())
        .bind(token_hash)
        .bind(expires_at)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(RefreshTokenRow {
            id,
            user_id: user_id.to_string(),
            token_hash: token_hash.to_string(),
            expires_at,
            revoked_at: None,
            created_at: now,
        })
    }

    pub async fn find_refresh(&self, token_hash: &str) -> Result<Option<RefreshTokenRow>, AppError> {
        let row = sqlx::query_as::<_, RefreshTokenRow>(
            "SELECT * FROM refresh_tokens WHERE token_hash = ?",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn revoke_refresh(&self, token_hash: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE refresh_tokens SET revoked_at = ? WHERE token_hash = ?")
            .bind(Utc::now())
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- sessions ----

    pub async fn add_session(
        &self,
        user_id: &Uuid,
        ip: Option<&str>,
        user_agent: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> Result<ActiveSessionRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO active_sessions (id, user_id, ip, user_agent, login_at, last_seen_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id.to_string())
        .bind(ip)
        .bind(user_agent)
        .bind(now)
        .bind(now)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(ActiveSessionRow {
            id,
            user_id: user_id.to_string(),
            ip: ip.map(str::to_string),
            user_agent: user_agent.map(str::to_string),
            login_at: now,
            last_seen_at: now,
            expires_at,
        })
    }

    pub async fn list_sessions(&self, user_id: &Uuid) -> Result<Vec<ActiveSessionRow>, AppError> {
        let rows = sqlx::query_as::<_, ActiveSessionRow>(
            "SELECT * FROM active_sessions WHERE user_id = ? AND expires_at > ? ORDER BY login_at DESC",
        )
        .bind(user_id.to_string())
        .bind(Utc::now())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn drop_session(&self, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM active_sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
