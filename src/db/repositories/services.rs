//! Service health repository.

use crate::db::models::service::ServiceHealthStateRow;
use crate::db::pool::DbPool;
use crate::error::AppError;

#[derive(Clone)]
pub struct ServiceHealthRepo {
    pool: DbPool,
}

impl ServiceHealthRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<ServiceHealthStateRow>, AppError> {
        let rows = sqlx::query_as::<_, ServiceHealthStateRow>(
            "SELECT * FROM service_health_state ORDER BY service_name ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn by_name(
        &self,
        name: &str,
    ) -> Result<Option<ServiceHealthStateRow>, AppError> {
        let row = sqlx::query_as::<_, ServiceHealthStateRow>(
            "SELECT * FROM service_health_state WHERE service_name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn record_failure(&self, name: &str) -> Result<ServiceHealthStateRow, AppError> {
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO service_health_state (service_name, consecutive_failures, last_failure_at, needs_attention)
             VALUES (?, 1, ?, 0)
             ON CONFLICT(service_name) DO UPDATE SET
                consecutive_failures = consecutive_failures + 1,
                last_failure_at = ?,
                needs_attention = CASE WHEN consecutive_failures + 1 >= 3 THEN 1 ELSE needs_attention END",
        )
        .bind(name)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        self.by_name(name)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("vanished")))
    }

    pub async fn record_success(&self, name: &str) -> Result<ServiceHealthStateRow, AppError> {
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO service_health_state (service_name, consecutive_failures, last_success_at, needs_attention)
             VALUES (?, 0, ?, 0)
             ON CONFLICT(service_name) DO UPDATE SET
                consecutive_failures = 0,
                last_success_at = ?,
                needs_attention = 0",
        )
        .bind(name)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        self.by_name(name)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("vanished")))
    }
}
