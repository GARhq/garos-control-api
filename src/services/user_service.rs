//! User business logic.

use crate::auth::jwt::{AuthUser, JwtService, Role};
use crate::auth::password;
use crate::db::models::user::{ActiveSessionRow, UserRow};
use crate::db::repositories::users::UserRepo;
use crate::domain::user::*;
use crate::error::AppError;
use validator::Validate;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserService {
    repo: UserRepo,
    jwt: Arc<JwtService>,
}

impl UserService {
    pub fn new(repo: UserRepo, jwt: Arc<JwtService>) -> Self {
        Self { repo, jwt }
    }

    pub fn jwt(&self) -> &JwtService {
        &self.jwt
    }

    pub fn repo(&self) -> &UserRepo {
        &self.repo
    }

    pub async fn login(
        &self,
        req: LoginRequest,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<LoginResponse, AppError> {
        let user = self
            .repo
            .by_username(&req.username)
            .await?
            .ok_or(AppError::Unauthorized)?;
        if user.status != "active" {
            return Err(AppError::Forbidden);
        }
        if let Some(until) = user.locked_until {
            if until > Utc::now() {
                return Err(AppError::Forbidden);
            }
        }
        let hash = user.password_hash.clone().ok_or(AppError::Unauthorized)?;
        if !password::verify_password(&req.password, &hash)? {
            let locked = self
                .repo
                .record_login_failure(&user.id(), 5)
                .await?;
            if locked {
                return Err(AppError::Forbidden);
            }
            return Err(AppError::Unauthorized);
        }
        self.repo.reset_login_failures(&user.id()).await?;
        let auth = AuthUser {
            id: user.id(),
            username: user.username.clone(),
            role: user.role(),
        };
        let pair = self.jwt.issue_pair(&auth)?;
        let now = Utc::now();
        self.repo
            .store_refresh(
                &user.id(),
                &JwtService::hash_refresh(&pair.refresh_token),
                now + self.jwt.refresh_ttl(),
            )
            .await?;
        self.repo
            .add_session(
                &user.id(),
                ip,
                user_agent,
                now + self.jwt.access_ttl(),
            )
            .await?;
        Ok(LoginResponse {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            token_type: pair.token_type,
            expires_in: pair.expires_in,
            user: UserBrief {
                id: user.id(),
                username: user.username,
                role: user.role,
            },
        })
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<LoginResponse, AppError> {
        let claims = self.jwt.verify(refresh_token, "refresh")?;
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("bad sub: {e}")))?;
        let hash = JwtService::hash_refresh(refresh_token);
        let stored = self.repo.find_refresh(&hash).await?;
        let stored = stored.ok_or(AppError::Unauthorized)?;
        if stored.revoked_at.is_some() {
            return Err(AppError::Unauthorized);
        }
        let user = self
            .repo
            .by_id(&user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;
        // Rotate: revoke old, issue new.
        self.repo.revoke_refresh(&hash).await?;
        let auth = AuthUser {
            id: user.id(),
            username: user.username.clone(),
            role: user.role(),
        };
        let pair = self.jwt.issue_pair(&auth)?;
        let now = Utc::now();
        self.repo
            .store_refresh(
                &user.id(),
                &JwtService::hash_refresh(&pair.refresh_token),
                now + self.jwt.refresh_ttl(),
            )
            .await?;
        Ok(LoginResponse {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            token_type: pair.token_type,
            expires_in: pair.expires_in,
            user: UserBrief {
                id: user.id(),
                username: user.username,
                role: user.role,
            },
        })
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<(), AppError> {
        let hash = JwtService::hash_refresh(refresh_token);
        self.repo.revoke_refresh(&hash).await?;
        // Also revoke in memory to prevent re-use within the same process.
        if let Ok(claims) = self.jwt.verify(refresh_token, "refresh") {
            self.jwt.revoke(&claims.jti);
        }
        Ok(())
    }

    pub async fn by_id(&self, id: &Uuid) -> Result<Option<UserRow>, AppError> {
        self.repo.by_id(id).await
    }

    pub async fn create(&self, req: UserCreate) -> Result<UserRow, AppError> {
        req.validate()?;
        if self.repo.by_username(&req.username).await?.is_some() {
            return Err(AppError::Conflict(format!("user {} exists", req.username)));
        }
        let hash = password::hash_password(&req.password, 65540)?;
        self.repo
            .create(
                &req.username,
                req.email.as_deref(),
                req.display_name.as_deref(),
                Some(&hash),
                &req.role,
                req.samba_dn.as_deref(),
            )
            .await
    }

    pub async fn update(&self, id: &Uuid, req: UserUpdate) -> Result<UserRow, AppError> {
        let existing = self
            .repo
            .by_id(id)
            .await?
            .ok_or(AppError::NotFound("user".into()))?;
        if let Some(r) = &req.role {
            Role::from_str(r)?;
        }
        let email = req.email.or(existing.email);
        let display_name = req.display_name.or(existing.display_name);
        let role = req.role.unwrap_or(existing.role);
        let samba_dn = req.samba_dn.or(existing.samba_dn);
        let quota_limit = req.quota_limit_bytes.or(existing.quota_limit_bytes);
        sqlx::query(
            "UPDATE users SET email = ?, display_name = ?, role = ?, samba_dn = ?, quota_limit_bytes = ?, updated_at = ? WHERE id = ?",
        )
        .bind(email)
        .bind(display_name)
        .bind(role)
        .bind(samba_dn)
        .bind(quota_limit)
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(self.repo.pool())
        .await?;
        self.repo.by_id(id).await?.ok_or(AppError::NotFound("user".into()))
    }

    pub async fn update_quota(
        &self,
        id: &Uuid,
        req: QuotaUpdateRequest,
    ) -> Result<UserRow, AppError> {
        req.validate()?;
        self.repo
            .update_quota(id, req.used_bytes, req.limit_bytes)
            .await?;
        self.repo.by_id(id).await?.ok_or(AppError::NotFound("user".into()))
    }

    pub async fn update_status(
        &self,
        id: &Uuid,
        req: StatusUpdateRequest,
    ) -> Result<UserRow, AppError> {
        req.validate()?;
        if !["active", "blocked", "disabled", "pending"].contains(&req.status.as_str()) {
            return Err(AppError::BadRequest("invalid status".into()));
        }
        self.repo.update_status(id, &req.status).await?;
        self.repo.by_id(id).await?.ok_or(AppError::NotFound("user".into()))
    }

    pub async fn soft_delete(&self, id: &Uuid) -> Result<(), AppError> {
        self.repo.soft_delete(id).await
    }

    pub async fn reset_password(
        &self,
        id: &Uuid,
        req: PasswordResetRequest,
    ) -> Result<(), AppError> {
        req.validate()?;
        let hash = password::hash_password(&req.new_password, 65540)?;
        self.repo.update_password(id, &hash).await
    }

    pub async fn sessions(&self, id: &Uuid) -> Result<Vec<ActiveSessionRow>, AppError> {
        self.repo.list_sessions(id).await
    }

    pub async fn unlock(&self, id: &Uuid) -> Result<(), AppError> {
        self.repo.unlock(id).await
    }

    pub async fn list(
        &self,
        search: Option<&str>,
        role: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserRow>, AppError> {
        self.repo.list(search, role, status, limit, offset).await
    }

    pub async fn stats(&self) -> Result<UserStats, AppError> {
        let s = self.repo.stats().await?;
        Ok(UserStats {
            total: s["total"].as_i64().unwrap_or(0),
            active: s["active"].as_i64().unwrap_or(0),
            blocked: s["blocked"].as_i64().unwrap_or(0),
        })
    }
}
