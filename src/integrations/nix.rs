//! Nix CLI wrapper. Validates inputs to prevent shell injection.

use crate::config::NixSettings;
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::timeout;

/// Validate a single argument against an allow-list.
fn validate_arg(arg: &str) -> Result<(), AppError> {
    if arg.is_empty() {
        return Err(AppError::BadRequest("empty argument".into()));
    }
    if arg.contains(';') || arg.contains('|') || arg.contains('$') || arg.contains('`') {
        return Err(AppError::BadRequest(format!(
            "disallowed characters in argument: {arg}"
        )));
    }
    // Reject control characters and most shell metachars.
    for c in arg.chars() {
        if c.is_control() {
            return Err(AppError::BadRequest(format!("control char in argument: {arg}")));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixBuildResult {
    pub store_path: String,
    pub derivation: String,
    pub elapsed_secs: f64,
}

#[async_trait]
pub trait Nix: Send + Sync {
    async fn run_nix(&self, args: &[&str]) -> Result<String, AppError>;
    async fn nix_build(&self, flake_ref: &str) -> Result<NixBuildResult, AppError>;
    async fn nix_flake_update(&self) -> Result<String, AppError>;
    async fn nixos_rebuild(&self, action: &str) -> Result<String, AppError>;
    async fn eval_expression(&self, expr: &str) -> Result<String, AppError>;
}

pub struct NixIntegration {
    settings: NixSettings,
    mock: bool,
}

impl NixIntegration {
    pub fn new(settings: NixSettings, mock: bool) -> Self {
        Self { settings, mock }
    }

    pub fn settings(&self) -> &NixSettings {
        &self.settings
    }
}

#[async_trait]
impl Nix for NixIntegration {
    async fn run_nix(&self, args: &[&str]) -> Result<String, AppError> {
        if self.mock {
            let joined = args.join(" ");
            tracing::debug!(target: "nix", %joined, "[mock] nix");
            return Ok(format!("/nix/store/mock-{}", uuid::Uuid::now_v7()));
        }
        for a in args {
            validate_arg(a)?;
        }
        let out = Command::new(&self.settings.binary_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        let output = timeout(self.settings.build_timeout(), out)
            .await
            .map_err(|_| AppError::ServiceUnavailable("nix timeout".into()))?
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Nix,
                message: format!("spawn: {e}"),
            })?;
        if !output.status.success() {
            return Err(AppError::Integration {
                kind: IntegrationKind::Nix,
                message: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn nix_build(&self, flake_ref: &str) -> Result<NixBuildResult, AppError> {
        validate_arg(flake_ref)?;
        if self.mock {
            return Ok(NixBuildResult {
                store_path: format!("/nix/store/mock-build-{flake_ref}"),
                derivation: format!("garos#mock-{flake_ref}"),
                elapsed_secs: 0.0,
            });
        }
        let started = std::time::Instant::now();
        let out = self.run_nix(&["build", flake_ref, "--no-link", "--print-out-paths"]).await?;
        Ok(NixBuildResult {
            store_path: out.trim().to_string(),
            derivation: flake_ref.to_string(),
            elapsed_secs: started.elapsed().as_secs_f64(),
        })
    }

    async fn nix_flake_update(&self) -> Result<String, AppError> {
        self.run_nix(&["flake", "update"]).await
    }

    async fn nixos_rebuild(&self, action: &str) -> Result<String, AppError> {
        validate_arg(action)?;
        if self.mock {
            return Ok(format!("[mock] nixos-rebuild {action} ok"));
        }
        self.run_nix(&["run", "nixpkgs#nixos-rebuild", "--", action, "--flake", "."]).await
    }

    async fn eval_expression(&self, expr: &str) -> Result<String, AppError> {
        validate_arg(expr)?;
        if self.mock {
            return Ok(format!("[mock] eval {expr}"));
        }
        self.run_nix(&["eval", "--impure", "--expr", expr]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_injection() {
        assert!(validate_arg("foo;rm -rf /").is_err());
        assert!(validate_arg("foo|cat").is_err());
        assert!(validate_arg("$(whoami)").is_err());
        assert!(validate_arg("`whoami`").is_err());
    }

    #[test]
    fn accepts_normal_path() {
        assert!(validate_arg("/nix/store/abc").is_ok());
        assert!(validate_arg(".#garos").is_ok());
    }
}
