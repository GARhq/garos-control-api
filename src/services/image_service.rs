//! Image business logic.

use crate::db::models::image::{ImageRow, ImageVersionRow};
use crate::db::repositories::audit::AuditRepo;
use crate::db::repositories::images::ImageRepo;
use crate::domain::image::*;
use crate::error::AppError;
use crate::integrations::nix::NixIntegration;
use crate::integrations::pxe::PxeIntegration;
use crate::realtime::events::Event;
use crate::realtime::hub::RealtimeHub;
use std::sync::Arc;
use uuid::Uuid;

pub struct ImageService {
    repo: ImageRepo,
    audit: AuditRepo,
    pxe: Arc<PxeIntegration>,
    nix: Arc<NixIntegration>,
    hub: RealtimeHub,
}

impl ImageService {
    pub fn new(
        repo: ImageRepo,
        audit: AuditRepo,
        pxe: Arc<PxeIntegration>,
        nix: Arc<NixIntegration>,
        hub: RealtimeHub,
    ) -> Self {
        Self {
            repo,
            audit,
            pxe,
            nix,
            hub,
        }
    }

    pub fn repo(&self) -> &ImageRepo {
        &self.repo
    }

    pub async fn list(&self) -> Result<Vec<ImageRow>, AppError> {
        self.repo.list().await
    }

    pub async fn by_id(&self, id: &Uuid) -> Result<Option<ImageRow>, AppError> {
        self.repo.by_id(id).await
    }

    pub async fn create(
        &self,
        req: ImageCreate,
        author_id: &Uuid,
    ) -> Result<ImageRow, AppError> {
        req.validate()?;
        if self.repo.by_name(&req.name).await?.is_some() {
            return Err(AppError::Conflict(format!("image {} exists", req.name)));
        }
        let packages_json = serde_json::to_string(&req.packages)?;
        let img = self
            .repo
            .create(
                &req.name,
                req.description.as_deref(),
                req.nixos_version.as_deref(),
                req.kernel.as_deref(),
                req.kernel_args.as_deref(),
                Some(&packages_json),
                req.custom_nix.as_deref(),
                Some(&author_id.to_string()),
                &req.version,
            )
            .await?;
        self.audit
            .record(
                Some(&author_id.to_string()),
                None,
                "image.create",
                Some("image"),
                Some(&img.id),
                None,
                Some(&serde_json::to_value(&img).unwrap_or_default()),
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(img)
    }

    pub async fn update(
        &self,
        id: &Uuid,
        req: ImageUpdate,
        actor: &str,
    ) -> Result<ImageRow, AppError> {
        let _ = req;
        let img = self
            .repo
            .by_id(id)
            .await?
            .ok_or(AppError::NotFound("image".into()))?;
        self.audit
            .record(
                None,
                Some(actor),
                "image.update",
                Some("image"),
                Some(&img.id),
                Some(&serde_json::to_value(&img).unwrap_or_default()),
                None,
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(img)
    }

    pub async fn delete(&self, id: &Uuid, actor: &str) -> Result<(), AppError> {
        let img = self
            .repo
            .by_id(id)
            .await?
            .ok_or(AppError::NotFound("image".into()))?;
        self.repo.delete(id).await?;
        self.audit
            .record(
                None,
                Some(actor),
                "image.delete",
                Some("image"),
                Some(&img.id),
                Some(&serde_json::to_value(&img).unwrap_or_default()),
                None,
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn start_build(&self, id: &Uuid) -> Result<ImageBuildStatus, AppError> {
        let img = self
            .repo
            .by_id(id)
            .await?
            .ok_or(AppError::NotFound("image".into()))?;
        self.repo.update_status(id, "building").await?;
        self.hub.publish(Event::ImageBuildProgress {
            image_id: img.id().unwrap_or_else(Uuid::nil),
            status: "building".into(),
            progress_pct: 0.0,
            at: chrono::Utc::now(),
        });
        // Kick off a real build via Nix (mock in dev).
        let _ = self.nix.nix_build(&format!(".#{}", img.name)).await?;
        self.repo.update_status(id, "ready").await?;
        Ok(ImageBuildStatus {
            image_id: img.id().unwrap_or_else(Uuid::nil),
            status: "ready".into(),
            progress_pct: 100.0,
            current_step: Some("completed".into()),
            log_tail: None,
            started_at: chrono::Utc::now(),
            finished_at: Some(chrono::Utc::now()),
        })
    }

    pub async fn build_status(&self, id: &Uuid) -> Result<ImageBuildStatus, AppError> {
        let img = self
            .repo
            .by_id(id)
            .await?
            .ok_or(AppError::NotFound("image".into()))?;
        Ok(ImageBuildStatus {
            image_id: img.id().unwrap_or_else(Uuid::nil),
            status: img.status.clone(),
            progress_pct: if img.status == "ready" { 100.0 } else { 50.0 },
            current_step: Some(img.status.clone()),
            log_tail: img.build_log.clone(),
            started_at: img.created_at,
            finished_at: img.published_at,
        })
    }

    pub async fn publish(&self, id: &Uuid, actor: &str) -> Result<(), AppError> {
        self.repo.publish(id).await?;
        self.audit
            .record(
                None,
                Some(actor),
                "image.publish",
                Some("image"),
                Some(&id.to_string()),
                None,
                None,
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        // Regenerate PXE menu.
        let images = self.repo.list().await?;
        let menu = self.pxe.render_menu(&images, &[]).await?;
        self.pxe.write_menu(&menu).await?;
        Ok(())
    }

    pub async fn unpublish(&self, id: &Uuid, actor: &str) -> Result<(), AppError> {
        self.repo.unpublish(id).await?;
        self.audit
            .record(
                None,
                Some(actor),
                "image.unpublish",
                Some("image"),
                Some(&id.to_string()),
                None,
                None,
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn list_versions(&self, id: &Uuid) -> Result<Vec<ImageVersionRow>, AppError> {
        self.repo.list_versions(id).await
    }

    pub async fn diff(&self, id: &Uuid, a: &str, b: &str) -> Result<ImageDiff, AppError> {
        let _ = (a, b);
        let img = self
            .repo
            .by_id(id)
            .await?
            .ok_or(AppError::NotFound("image".into()))?;
        Ok(ImageDiff {
            image_id: img.id().unwrap_or_else(Uuid::nil),
            version_a: a.into(),
            version_b: b.into(),
            packages_added: vec!["curl".into(), "jq".into()],
            packages_removed: vec!["wget".into()],
            nix_diff: Some("(mock) 2 packages added, 1 removed".into()),
        })
    }

    pub async fn stations(&self, id: &Uuid) -> Result<serde_json::Value, AppError> {
        let _ = id;
        Ok(serde_json::json!({ "count": 0, "items": [] }))
    }
}

impl ImageRow {
    pub fn id(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.id).ok()
    }
}
