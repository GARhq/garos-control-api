//! JWT RS256 token issuance and verification.

use crate::auth::password;
use crate::config::AuthSettings;
use crate::error::AppError;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// User role hierarchy: `admin > operator > user`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Operator,
    Admin,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Operator => "operator",
            Self::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "user" => Ok(Self::User),
            "operator" => Ok(Self::Operator),
            "admin" => Ok(Self::Admin),
            other => Err(AppError::BadRequest(format!("unknown role: {other}"))),
        }
    }
}

/// JWT claims (RS256).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject = user_id
    pub sub: String,
    /// Username for logging.
    pub username: String,
    /// Role
    pub role: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiration (epoch seconds)
    pub exp: i64,
    /// Issued at (epoch seconds)
    pub iat: i64,
    /// JWT ID (random)
    pub jti: String,
    /// `access` | `refresh`
    #[serde(default = "default_kind")]
    pub kind: String,
}

fn default_kind() -> String {
    "access".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
}

/// JWT service: holds the keypair and a revocation set for refresh tokens.
pub struct JwtService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    settings: AuthSettings,
    /// In-memory revocation set of refresh `jti`s.
    revoked: RwLock<HashSet<String>>,
}

impl std::fmt::Debug for JwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtService")
            .field("issuer", &self.settings.jwt_issuer)
            .field("audience", &self.settings.jwt_audience)
            .field("revoked_count", &self.revoked.read().map(|s| s.len()).unwrap_or(0))
            .finish()
    }
}

impl JwtService {
    /// Build a new `JwtService` from configuration. If a private key path is
    /// provided, it is loaded; otherwise a static secret is used (HS256 dev
    /// fallback).
    pub fn new(settings: AuthSettings) -> Result<Self, AppError> {
        let (encoding, decoding) = if let Some(path) = settings.jwt_private_key_path.as_ref() {
            let pem = std::fs::read(path)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("read private key: {e}")))?;
            let enc = EncodingKey::from_rsa_pem(&pem)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("parse private key: {e}")))?;
            let dec = if let Some(pub_path) = settings.jwt_public_key_path.as_ref() {
                let pub_pem = std::fs::read(pub_path).map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("read public key: {e}"))
                })?;
                DecodingKey::from_rsa_pem(&pub_pem)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("parse public key: {e}")))?
            } else {
                DecodingKey::from_rsa_components(&[], &[], &pem)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("derive public key: {e}")))?
            };
            (enc, dec)
        } else if let Some(secret) = settings.jwt_secret.as_deref() {
            let enc = EncodingKey::from_secret(secret.as_bytes());
            let dec = DecodingKey::from_secret(secret.as_bytes());
            (enc, dec)
        } else {
            return Err(AppError::Internal(anyhow::anyhow!(
                "auth.jwt_secret or auth.jwt_private_key_path must be set"
            )));
        };

        Ok(Self {
            encoding,
            decoding,
            settings,
            revoked: RwLock::new(HashSet::new()),
        })
    }

    pub fn issue(
        &self,
        user: &AuthUser,
        kind: &str,
        ttl: std::time::Duration,
    ) -> Result<(String, Claims), AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("clock: {e}")))?
            .as_secs() as i64;
        let jti = Uuid::now_v7().to_string();
        let claims = Claims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            role: user.role.as_str().to_string(),
            iss: self.settings.jwt_issuer.clone(),
            aud: self.settings.jwt_audience.clone(),
            exp: now + ttl.as_secs() as i64,
            iat: now,
            jti,
            kind: kind.into(),
        };
        let mut header = Header::new(Algorithm::RS256);
        if self.settings.jwt_private_key_path.is_none() {
            header.alg = Algorithm::HS256;
        }
        let token = encode(&header, &claims, &self.encoding)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("jwt encode: {e}")))?;
        Ok((token, claims))
    }

    pub fn issue_pair(&self, user: &AuthUser) -> Result<TokenPair, AppError> {
        let (access, _) = self.issue(user, "access", self.settings.access_ttl())?;
        let (refresh, _) = self.issue(user, "refresh", self.settings.refresh_ttl())?;
        Ok(TokenPair {
            access_token: access,
            refresh_token: refresh,
            token_type: "Bearer".to_string(),
            expires_in: self.settings.access_ttl_secs as i64,
        })
    }

    pub fn verify(&self, token: &str, expected_kind: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::new(if self.settings.jwt_private_key_path.is_some() {
            Algorithm::RS256
        } else {
            Algorithm::HS256
        });
        validation.set_issuer(&[&self.settings.jwt_issuer]);
        validation.set_audience(&[&self.settings.jwt_audience]);
        let data = decode::<Claims>(token, &self.decoding, &validation)
            .map_err(|e| AppError::Unauthorized)?;
        if data.claims.kind != expected_kind {
            return Err(AppError::Unauthorized);
        }
        if data.claims.kind == "refresh" && self.is_revoked(&data.claims.jti) {
            return Err(AppError::Unauthorized);
        }
        Ok(data.claims)
    }

    pub fn revoke(&self, jti: &str) {
        if let Ok(mut g) = self.revoked.write() {
            g.insert(jti.to_string());
        }
    }

    pub fn is_revoked(&self, jti: &str) -> bool {
        self.revoked.read().map(|g| g.contains(jti)).unwrap_or(false)
    }

    /// Derive a stable hash of a refresh token (for storage).
    pub fn hash_refresh(token: &str) -> String {
        crate::auth::password::sha256_hex(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with_secret(s: &str) -> AuthSettings {
        AuthSettings {
            jwt_secret: Some(s.into()),
            jwt_private_key_path: None,
            jwt_public_key_path: None,
            jwt_issuer: "garos-test".into(),
            jwt_audience: "garos-api".into(),
            access_token_ttl_secs: 60,
            refresh_token_ttl_secs: 3600,
            argon2_cost: 4096,
            idempotency_ttl_secs: 60,
        }
    }

    #[test]
    fn issue_and_verify_access() {
        let svc = JwtService::new(settings_with_secret("super-secret")).unwrap();
        let user = AuthUser {
            id: Uuid::now_v7(),
            username: "admin".into(),
            role: Role::Admin,
        };
        let pair = svc.issue_pair(&user).unwrap();
        let claims = svc.verify(&pair.access_token, "access").unwrap();
        assert_eq!(claims.username, "admin");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    fn refresh_revocation_works() {
        let svc = JwtService::new(settings_with_secret("super-secret")).unwrap();
        let user = AuthUser {
            id: Uuid::now_v7(),
            username: "u".into(),
            role: Role::User,
        };
        let pair = svc.issue_pair(&user).unwrap();
        let claims = svc.verify(&pair.refresh_token, "refresh").unwrap();
        svc.revoke(&claims.jti);
        assert!(svc.verify(&pair.refresh_token, "refresh").is_err());
    }
}
