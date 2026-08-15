//! Application error type and HTTP response mapping.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Result alias using [`AppError`].
pub type AppResult<T> = Result<T, AppError>;

/// Categorisation of an integration call. Used to disambiguate metrics
/// (`garos_integration_errors_total{kind="nix|samba|..."}`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationKind {
    Nix,
    Samba,
    Btrfs,
    Nftables,
    Systemd,
    Journald,
    Wol,
    Pxe,
    Websocket,
    Smtp,
    Http,
    Other,
}

impl IntegrationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nix => "nix",
            Self::Samba => "samba",
            Self::Btrfs => "btrfs",
            Self::Nftables => "nftables",
            Self::Systemd => "systemd",
            Self::Journald => "journald",
            Self::Wol => "wol",
            Self::Pxe => "pxe",
            Self::Websocket => "websocket",
            Self::Smtp => "smtp",
            Self::Http => "http",
            Self::Other => "other",
        }
    }
}

/// A single field-level validation error.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}

/// All application errors. The HTTP layer converts these into
/// structured JSON responses.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("validation failed")]
    Validation(Vec<FieldError>),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("integration failure ({kind:?}): {message}")]
    Integration { kind: IntegrationKind, message: String },

    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("rate limit exceeded; retry in {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("internal error")]
    Internal(#[from] anyhow::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("ldap error: {0}")]
    Ldap(#[from] ldap3::LdapError),

    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),
}

impl AppError {
    /// HTTP status code for this error.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) | Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Integration { .. } => StatusCode::BAD_GATEWAY,
            Self::Timeout(_) => StatusCode::GATEWAY_TIMEOUT,
            Self::Io(_) | Self::Serde(_) | Self::Sqlx(_) | Self::Ldap(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// Stable string code for clients.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "not_found",
            Self::BadRequest(_) => "bad_request",
            Self::Validation(_) => "validation_error",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::Conflict(_) => "conflict",
            Self::Integration { .. } => "integration_error",
            Self::ServiceUnavailable(_) => "service_unavailable",
            Self::RateLimited { .. } => "rate_limited",
            Self::Internal(_) => "internal_error",
            Self::Io(_) => "io_error",
            Self::Serde(_) => "serialization_error",
            Self::Sqlx(_) => "database_error",
            Self::Ldap(_) => "ldap_error",
            Self::Timeout(_) => "timeout",
        }
    }

    /// Public, safe message (no internal details).
    pub fn public_message(&self) -> String {
        match self {
            Self::Internal(_)
            | Self::Io(_)
            | Self::Serde(_)
            | Self::Sqlx(_)
            | Self::Ldap(_) => "Internal server error".to_string(),
            Self::Integration { kind, .. } => format!("Upstream {} call failed", kind.as_str()),
            other => other.to_string(),
        }
    }
}

impl From<tokio::time::error::Elapsed> for AppError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        // tokio 1.41+ made `Elapsed` opaque: no public accessor for the
        // elapsed Duration. Caller can use the original deadline via context.
        Self::Timeout(std::time::Duration::ZERO)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        Self::Internal(anyhow::Error::new(e))
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errs: validator::ValidationErrors) -> Self {
        let fields = errs
            .into_errors()
            .into_iter()
            .flat_map(|(field, kind)| match kind {
                validator::ValidationErrorsKind::Field(errs) => errs
                    .into_iter()
                    .map(|e| FieldError {
                        field: field.to_string(),
                        code: e.code.to_string(),
                        message: e
                            .message
                            .map_or_else(|| e.code.to_string(), |m| m.to_string()),
                    })
                    .collect::<Vec<_>>(),
                validator::ValidationErrorsKind::List(map) => map
                    .into_iter()
                    .flat_map(|(idx, errs)| {
                        errs.into_errors().into_iter().map(move |(field, kind)| {
                            let msg = match kind {
                                validator::ValidationErrorsKind::Field(fs) => fs
                                    .into_iter()
                                    .next()
                                    .map(|e| {
                                        e.message
                                            .map_or_else(|| e.code.to_string(), |m| m.to_string())
                                    })
                                    .unwrap_or_default(),
                                _ => "invalid".to_string(),
                            };
                            FieldError {
                                field: format!("{field}[{idx}]"),
                                code: "nested".into(),
                                message: msg,
                            }
                        })
                    })
                    .collect::<Vec<_>>(),
                validator::ValidationErrorsKind::Struct(_) => vec![FieldError {
                    field: field.to_string(),
                    code: "struct".into(),
                    message: "invalid nested object".into(),
                }],
            })
            .collect();
        Self::Validation(fields)
    }
}

/// JSON body returned for errors.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorDetail {
    /// Stable string code, e.g. `"not_found"`.
    pub code: String,
    /// Public, human-readable message.
    pub message: String,
    /// Per-field validation errors (only for `validation_error`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<FieldError>>,
    /// Trace ID (request_id, UUID v7) — present on every error.
    pub trace_id: Uuid,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code();
        let message = self.public_message();
        let fields = match &self {
            Self::Validation(fs) => Some(fs.clone()),
            _ => None,
        };

        // Log internal errors with full context, public ones at info.
        if status.is_server_error() {
            tracing::error!(error.code = %code, error.detail = %self, "request failed");
        } else {
            tracing::info!(error.code = %code, error.message = %message, "request rejected");
        }

        // Best-effort trace id from current span; otherwise a fresh one.
        let trace_id = crate::middleware::request_id::current_trace_id().unwrap_or_else(Uuid::now_v7);

        let body = ErrorBody {
            error: ErrorDetail {
                code: code.to_string(),
                message,
                fields,
                trace_id,
            },
        };

        let mut resp = (status, Json(body)).into_response();
        if let Self::RateLimited { retry_after_secs } = &self {
            if let Ok(v) = retry_after_secs.to_string().parse() {
                resp.headers_mut().insert(axum::http::header::RETRY_AFTER, v);
            }
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn not_found_maps_to_404() {
        let e = AppError::NotFound("foo".into());
        assert_eq!(e.status(), StatusCode::NOT_FOUND);
        assert_eq!(e.code(), "not_found");
    }

    #[test]
    fn internal_error_hides_details() {
        let e = AppError::Internal(anyhow::anyhow!("secret stack"));
        assert_eq!(e.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(e.public_message(), "Internal server error");
    }

    #[test]
    fn rate_limited_includes_retry_after() {
        let resp = AppError::RateLimited { retry_after_secs: 7 }.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            resp.headers().get(axum::http::header::RETRY_AFTER).unwrap(),
            "7"
        );
    }

    #[test]
    fn validation_to_field_errors() {
        #[derive(validator::Validate)]
        struct S {
            #[validate(length(min = 3))]
            name: String,
        }
        let s = S { name: "a".to_string() };
        let err: AppError = s.validate().unwrap_err().into();
        assert!(matches!(err, AppError::Validation(ref fs) if !fs.is_empty()));
    }
}
