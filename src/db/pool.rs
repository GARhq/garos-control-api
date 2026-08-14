//! Database pool construction.
//!
//! The default backend is SQLite (development); PostgreSQL support can be
//! re-introduced via `sqlx::Any` once the `sqlx/any` feature is enabled in
//! `Cargo.toml`. For now we only build with `sqlite`, so the pool is plain
//! `sqlx::SqlitePool`.

use crate::config::DatabaseSettings;
use crate::error::AppError;
use std::time::Duration;

pub type DbPool = sqlx::SqlitePool;

/// Detect backend kind from URL.
///
/// With only the `sqlite` sqlx feature compiled in we always return `Sqlite`.
/// When `sqlx/any` is reintroduced this can be expanded to dispatch on URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnyKind {
    Sqlite,
    Postgres,
}

pub fn detect_kind(url: &str) -> AnyKind {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        AnyKind::Postgres
    } else {
        AnyKind::Sqlite
    }
}

/// Build the global connection pool.
pub async fn build_pool(settings: &DatabaseSettings) -> Result<DbPool, AppError> {
    let kind = detect_kind(&settings.url);
    if matches!(kind, AnyKind::Postgres) {
        return Err(AppError::Internal(anyhow::anyhow!(
            "postgres backend requested but this build only has the `sqlite` sqlx feature enabled"
        )));
    }

    let opts = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .acquire_timeout(settings.acquire_timeout())
        .idle_timeout(Some(Duration::from_secs(60 * 5)))
        .max_lifetime(Some(Duration::from_secs(60 * 60)))
        .test_before_acquire(true)
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                // SQLite-only PRAGMAs (safe no-ops if the backend ever changes).
                let _ = sqlx::query("PRAGMA journal_mode = WAL;")
                    .execute(&mut *conn)
                    .await;
                let _ = sqlx::query("PRAGMA foreign_keys = ON;")
                    .execute(&mut *conn)
                    .await;
                let _ = sqlx::query("PRAGMA synchronous = NORMAL;")
                    .execute(&mut *conn)
                    .await;
                Ok(())
            })
        });

    let pool = opts
        .connect(&settings.url)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("db connect: {e}")))?;

    Ok(pool)
}

/// Run all embedded migrations.
pub async fn run_migrations(pool: &DbPool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("migrate: {e}")))
}

/// Helper for tests: build an in-memory SQLite pool.
pub async fn memory_pool() -> Result<DbPool, AppError> {
    let opts = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .test_before_acquire(false)
        .acquire_timeout(Duration::from_secs(2));
    let pool = opts
        .connect("sqlite::memory:")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("memory pool: {e}")))?;
    Ok(pool)
}

/// Test connection URL is parseable.
#[allow(dead_code)]
pub fn parse_url(url: &str) -> Result<(), AppError> {
    if url.starts_with("postgres://")
        || url.starts_with("postgresql://")
        || url.starts_with("sqlite:")
    {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!("unsupported db url: {url}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_sqlite() {
        assert_eq!(detect_kind("sqlite::memory:"), AnyKind::Sqlite);
        assert_eq!(detect_kind("sqlite:///tmp/foo.db"), AnyKind::Sqlite);
    }

    #[test]
    fn detect_postgres() {
        assert_eq!(detect_kind("postgres://localhost/foo"), AnyKind::Postgres);
        assert_eq!(detect_kind("postgresql://localhost/foo"), AnyKind::Postgres);
    }

    #[tokio::test]
    async fn memory_pool_works() {
        let pool = memory_pool().await.unwrap();
        let row: (i64,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, 1);
    }
}