//! systemd service manager business logic.

use crate::db::repositories::services::ServiceHealthRepo;
use crate::domain::service::{LogLine, ServiceHealth, ServiceView};
use crate::error::AppError;
use crate::integrations::systemd::{Systemd, SystemdIntegration};
use std::sync::Arc;

pub struct ServiceManager {
    systemd: Arc<SystemdIntegration>,
    health_repo: ServiceHealthRepo,
}

impl ServiceManager {
    pub fn new(systemd: Arc<SystemdIntegration>, health_repo: ServiceHealthRepo) -> Self {
        Self {
            systemd,
            health_repo,
        }
    }

    pub async fn list(&self) -> Result<Vec<ServiceView>, AppError> {
        self.systemd.list_units().await
    }

    pub async fn by_name(&self, name: &str) -> Result<ServiceView, AppError> {
        self.systemd.unit(name).await
    }

    pub async fn start(&self, name: &str) -> Result<(), AppError> {
        self.systemd.start(name).await
    }

    pub async fn stop(&self, name: &str) -> Result<(), AppError> {
        self.systemd.stop(name).await
    }

    pub async fn restart(&self, name: &str) -> Result<(), AppError> {
        self.systemd.restart(name).await
    }

    pub async fn logs(
        &self,
        name: &str,
        lines: u32,
        since: Option<&str>,
        until: Option<&str>,
        priority: Option<&str>,
    ) -> Result<Vec<LogLine>, AppError> {
        self.systemd
            .logs(name, lines, since, until, priority)
            .await
    }

    pub async fn health(&self, name: &str) -> Result<ServiceHealth, AppError> {
        self.systemd.health(name, &self.health_repo).await
    }
}
