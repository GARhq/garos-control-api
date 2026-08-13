//! `CurrentUser` axum extractor.

use crate::auth::jwt::Claims;
use crate::error::AppError;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

/// Authenticated user, injected by the `require_auth` middleware.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
    pub role: crate::auth::jwt::Role,
    pub jti: String,
}

impl CurrentUser {
    pub fn from_claims(c: &Claims) -> Result<Self, AppError> {
        Ok(Self {
            id: Uuid::parse_str(&c.sub)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("bad sub uuid: {e}")))?,
            username: c.username.clone(),
            role: crate::auth::jwt::Role::from_str(&c.role)?,
            jti: c.jti.clone(),
        })
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}
