//! Images repository.

use crate::db::models::image::{ImageRow, ImageVersionRow};
use crate::db::pool::DbPool;
use crate::error::AppError;
use uuid::Uuid;

#[derive(Clone)]
pub struct ImageRepo {
    pool: DbPool,
}

impl ImageRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn by_id(&self, id: &Uuid) -> Result<Option<ImageRow>, AppError> {
        let row = sqlx::query_as::<_, ImageRow>("SELECT * FROM images WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn by_name(&self, name: &str) -> Result<Option<ImageRow>, AppError> {
        let row = sqlx::query_as::<_, ImageRow>("SELECT * FROM images WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn create(
        &self,
        name: &str,
        description: Option<&str>,
        nixos_version: Option<&str>,
        kernel: Option<&str>,
        kernel_args: Option<&str>,
        packages_json: Option<&str>,
        custom_nix: Option<&str>,
        author_id: Option<&str>,
        version: &str,
    ) -> Result<ImageRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO images
               (id, name, description, nixos_version, kernel, kernel_args, status, packages_json, custom_nix, author_id, version, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, 'draft', ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(nixos_version)
        .bind(kernel)
        .bind(kernel_args)
        .bind(packages_json)
        .bind(custom_nix)
        .bind(author_id)
        .bind(version)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        self.by_id(&Uuid::parse_str(&id).unwrap())
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("image vanished after insert")))
    }

    pub async fn update_status(&self, id: &Uuid, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE images SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(chrono::Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn publish(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE images SET status = 'published', published_at = ?, updated_at = ? WHERE id = ?")
            .bind(chrono::Utc::now())
            .bind(chrono::Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn unpublish(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE images SET status = 'ready', updated_at = ? WHERE id = ?")
            .bind(chrono::Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_size(&self, id: &Uuid, size_mb: i64) -> Result<(), AppError> {
        sqlx::query("UPDATE images SET size_mb = ?, updated_at = ? WHERE id = ?")
            .bind(size_mb)
            .bind(chrono::Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn append_log(&self, id: &Uuid, line: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE images SET build_log = COALESCE(build_log, '') || ? || E'\\n', updated_at = ? WHERE id = ?")
            .bind(line)
            .bind(chrono::Utc::now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<ImageRow>, AppError> {
        let rows = sqlx::query_as::<_, ImageRow>("SELECT * FROM images ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn delete(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM images WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- versions ----

    pub async fn add_version(
        &self,
        image_id: &Uuid,
        version: &str,
        size_mb: Option<i64>,
        packages_json: Option<&str>,
        custom_nix: Option<&str>,
        change_summary: Option<&str>,
        author_id: Option<&str>,
    ) -> Result<ImageVersionRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO image_versions (id, image_id, version, size_mb, packages_json, custom_nix, change_summary, author_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(image_id.to_string())
        .bind(version)
        .bind(size_mb)
        .bind(packages_json)
        .bind(custom_nix)
        .bind(change_summary)
        .bind(author_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(ImageVersionRow {
            id,
            image_id: image_id.to_string(),
            version: version.to_string(),
            size_mb,
            packages_json: packages_json.map(str::to_string),
            custom_nix: custom_nix.map(str::to_string),
            change_summary: change_summary.map(str::to_string),
            author_id: author_id.map(str::to_string),
            created_at: now,
        })
    }

    pub async fn list_versions(&self, image_id: &Uuid) -> Result<Vec<ImageVersionRow>, AppError> {
        let rows = sqlx::query_as::<_, ImageVersionRow>(
            "SELECT * FROM image_versions WHERE image_id = ? ORDER BY created_at DESC",
        )
        .bind(image_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
