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

/// Middleware factory: require a minimum [`Role`].
pub fn require_role(min: Role) -> impl Clone + Send + Sync + 'static {
    let min = min;
    move |State(state): State<AppState>, req: Request, next: Next| {
        let min = min;
        async move {
            let token = extract_token(&req)?;
            let claims = state.jwt.verify(&token, "access")?;
            let user = CurrentUser::from_claims(&claims)?;
            let role = user.role;
            if role < min {
                return Err(AppError::Forbidden);
            }
            let mut req = req;
            req.extensions_mut().insert(user);
            Ok::<_, AppError>(next.run(req).await)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
