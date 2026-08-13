//! Database pool construction.
//!
//! We use `sqlx::Any` so the same binary can speak to SQLite (default, dev) and
//! PostgreSQL (`features.enable_postgres = true`) without recompilation. This
//! trades a little runtime indirection for a much simpler build matrix.

use crate::config::DatabaseSettings;
use crate::error::AppError;
use sqlx::any::install_default_drivers;
use sqlx::any::AnyPoolOptions;
use sqlx::any::AnyKind;
use std::str::FromStr;
use std::time::Duration;

pub type DbPool = sqlx::Pool<sqlx::Any>;

/// Detect backend kind from URL.
pub fn detect_kind(url: &str) -> AnyKind {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        AnyKind::Postgres
    } else {
        AnyKind::Sqlite
    }
}

/// Build the global connection pool.
pub async fn build_pool(settings: &DatabaseSettings) -> Result<DbPool, AppError> {
    install_default_drivers();
    let kind = detect_kind(&settings.url);

    let url = match kind {
        // sqlx::Any requires sqlite::memory: with three slashes
        AnyKind::Sqlite if settings.url == "sqlite::memory:" => {
            "sqlite::memory:".to_string()
        }
        AnyKind::Sqlite => settings.url.clone(),
        AnyKind::Postgres => settings.url.clone(),
        _ => settings.url.clone(),
    };

    let opts = AnyPoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .acquire_timeout(settings.acquire_timeout())
        .idle_timeout(Some(Duration::from_secs(60 * 5)))
        .max_lifetime(Some(Duration::from_secs(60 * 60)))
        .test_before_acquire(true)
        .after_connect({
            let kind = kind;
            move |conn, _meta| {
                Box::pin(async move {
                    match kind {
                        AnyKind::Sqlite => {
                            sqlx::query("PRAGMA journal_mode = WAL;")
                                .execute(&mut *conn)
                                .await
                                .ok();
                            sqlx::query("PRAGMA foreign_keys = ON;")
                                .execute(&mut *conn)
                                .await
                                .ok();
                            sqlx::query("PRAGMA synchronous = NORMAL;")
                                .execute(&mut *conn)
                                .await
                                .ok();
                        }
                        AnyKind::Postgres => {
                            sqlx::query("SET client_min_messages = WARNING;")
                                .execute(&mut *conn)
                                .await
                                .ok();
                        }
                        _ => {}
                    }
                    Ok(())
                })
            }
        });

    let pool = opts
        .connect(&url)
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
    install_default_drivers();
    let opts = AnyPoolOptions::new()
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

/// Used by integration tests.
#[allow(dead_code)]
pub fn _from_str_used() {
    let _ = AnyKind::from_str("sqlite");
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
        assert_eq!(
            detect_kind("postgres://localhost/foo"),
            AnyKind::Postgres
        );
        assert_eq!(
            detect_kind("postgresql://localhost/foo"),
            AnyKind::Postgres
        );
    }

    #[tokio::test]
    async fn memory_pool_works() {
        let pool = memory_pool().await.unwrap();
        let row: (i64,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, 1);
    }
}
