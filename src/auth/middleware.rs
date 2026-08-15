//! Auth middleware: validates the `Authorization: Bearer <jwt>` header
//! and stores the [`CurrentUser`] in request extensions.

use crate::auth::extractor::CurrentUser;
use crate::auth::jwt::JwtService;
use crate::auth::jwt::Role;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

const BEARER: &str = "Bearer ";

fn extract_token(req: &Request) -> Result<String, AppError> {
    let h = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let trimmed = h.trim();
    let token = trimmed
        .strip_prefix(BEARER)
        .ok_or(AppError::Unauthorized)?
        .trim();
    if token.is_empty() {
        return Err(AppError::Unauthorized);
    }
    Ok(token.to_string())
}

/// Allow a `?token=...` query-param fallback (used for WebSocket).
pub fn extract_token_or_query(req: &Request) -> Result<String, AppError> {
    if let Ok(t) = extract_token(req) {
        return Ok(t);
    }
    if let Some(q) = req.uri().query() {
        for pair in q.split('&') {
            if let Some(rest) = pair.strip_prefix("token=") {
                return Ok(rest.to_string());
            }
        }
    }
    Err(AppError::Unauthorized)
}

/// Middleware: verify JWT and inject [`CurrentUser`].
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_token(&req)?;
    let claims = state.jwt.verify(&token, "access")?;
    let user = CurrentUser::from_claims(&claims)?;
    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

/// Middleware: require the JWT to carry the `Admin` role.
///
/// Applied to admin-only routes (user provisioning, firewall policy,
/// panic-mode, storage writes — see Wave 6 critical #1).
/// Because it is an `async fn` with the proper signature, it can be
/// consumed by `axum::middleware::from_fn_with_state(state, require_role)`.
pub async fn require_role(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_token(&req)?;
    let claims = state.jwt.verify(&token, "access")?;
    let user = CurrentUser::from_claims(&claims)?;
    if user.role < Role::Admin {
        return Err(AppError::Forbidden);
    }
    let mut req = req;
    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt::Role;

    /// Sanity: the role ordering used by `require_role` (Operator < Admin)
    /// must hold. The `require_role` middleware rejects any caller whose
    /// role is below `Role::Admin`, so we depend on this Ord contract.
    #[test]
    fn operator_ranks_below_admin() {
        assert!(Role::Operator < Role::Admin);
    }

    /// Regression for Wave 6 critical #1: a plain user must also be rejected
    /// when hitting an admin-only route (was the privilege-escalation vector).
    #[test]
    fn user_ranks_below_admin() {
        assert!(Role::User < Role::Admin);
    }

    /// Admin equals Admin (boundary of the >= comparison in require_role).
    #[test]
    fn admin_is_ge_admin() {
        assert!(Role::Admin >= Role::Admin);
    }

    #[test]
    fn parse_bearer() {
        let req = Request::builder()
            .header("Authorization", "Bearer abc.def.ghi")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(extract_token(&req).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn missing_header_is_unauthorized() {
        let req = Request::builder().body(axum::body::Body::empty()).unwrap();
        assert!(matches!(extract_token(&req), Err(AppError::Unauthorized)));
    }
}
