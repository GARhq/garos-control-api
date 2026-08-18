//! Node business logic.

use crate::db::models::node::NodeRow;
use crate::db::repositories::audit::AuditRepo;
use crate::db::repositories::nodes::NodeRepo;
use crate::domain::node::*;
use crate::error::AppError;
use crate::integrations::nix::NixIntegration;
use crate::integrations::pxe::PxeIntegration;
use crate::integrations::wol::{Wol, WolIntegration};
use crate::realtime::events::Event;
use crate::realtime::hub::RealtimeHub;
use validator::Validate;
use std::sync::Arc;
use uuid::Uuid;

pub struct NodeService {
    repo: NodeRepo,
    audit: AuditRepo,
    wol: Arc<WolIntegration>,
    pxe: Arc<PxeIntegration>,
    nix: Arc<NixIntegration>,
    hub: RealtimeHub,
}

impl NodeService {
    pub fn new(
        repo: NodeRepo,
        audit: AuditRepo,
        wol: Arc<WolIntegration>,
        pxe: Arc<PxeIntegration>,
        nix: Arc<NixIntegration>,
        hub: RealtimeHub,
    ) -> Self {
        Self {
            repo,
            audit,
            wol,
            pxe,
            nix,
            hub,
        }
    }

    pub fn repo(&self) -> &NodeRepo {
        &self.repo
    }

    pub fn audit(&self) -> &AuditRepo {
        &self.audit
    }

    pub async fn by_id(&self, id: &Uuid) -> Result<Option<NodeRow>, AppError> {
        self.repo.by_id(id).await
    }

    pub async fn by_mac(&self, mac: &str) -> Result<Option<NodeRow>, AppError> {
        self.repo.by_mac(mac).await
    }

    pub async fn list(
        &self,
        status: Option<&str>,
        image_id: Option<&str>,
        search: Option<&str>,
        limit: i64,
        offset: i64,
        sort: &str,
        order: &str,
    ) -> Result<Vec<NodeRow>, AppError> {
        self.repo
            .list(status, image_id, search, limit, offset, sort, order)
            .await
    }

    pub async fn stats(&self) -> Result<NodeStats, AppError> {
        let s = self.repo.stats().await?;
        let total = s["total"].as_i64().unwrap_or(0);
        let by_status = s["byStatus"]
            .as_object()
            .map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_i64().unwrap_or(0)))
                    .collect()
            })
            .unwrap_or_default();
        Ok(NodeStats { total, by_status })
    }

    pub async fn heartbeat(
        &self,
        mac: &str,
        req: HeartbeatRequest,
    ) -> Result<NodeRow, AppError> {
        let node = self
            .repo
            .upsert_heartbeat(
                mac,
                req.ip.as_deref(),
                req.hostname.as_deref(),
                req.cpu_temp_c,
                req.cpu_usage_pct,
                req.mem_usage_pct,
                req.ping_ms,
                req.nfs_latency_ms,
                req.status.as_deref(),
            )
            .await?;
        self.hub.publish(Event::NodeHeartbeat {
            mac: mac.into(),
            cpu_temp_c: req.cpu_temp_c,
            cpu_usage_pct: req.cpu_usage_pct,
            mem_usage_pct: req.mem_usage_pct,
            at: chrono::Utc::now(),
        });
        Ok(node)
    }

    pub async fn wol(&self, mac: &str) -> Result<WolResult, AppError> {
        if !MacAddress::is_valid(mac) {
            return Err(AppError::BadRequest("invalid MAC".into()));
        }
        let r = self.wol.send(mac).await?;
        Ok(WolResult {
            mac: r.mac,
            sent_at: r.sent_at,
            broadcast: r.broadcast,
        })
    }

    pub async fn reboot(
        &self,
        mac: &str,
        actor: &str,
        trace_id: Option<&str>,
    ) -> Result<NodeRow, AppError> {
        let _ = (actor, trace_id);
        // We pretend: status -> rebooting. The actual remote reboot is
        // platform-specific; we record the intent.
        let node = self
            .repo
            .by_mac(mac)
            .await?
            .ok_or(AppError::NotFound(format!("node {mac}")))?;
        self.repo.set_status(mac, "rebooting").await?;
        let updated = self
            .repo
            .by_mac(mac)
            .await?
            .ok_or(AppError::NotFound(format!("node {mac}")))?;
        self.hub.publish(Event::NodeStatusChanged {
            id: updated.parse_id().unwrap_or_else(Uuid::nil),
            mac: mac.into(),
            status: "rebooting".into(),
            at: chrono::Utc::now(),
        });
        // Audit
        self.audit
            .record(
                None,
                Some(actor),
                "node.reboot",
                Some("node"),
                Some(mac),
                Some(&serde_json::to_value(&node).unwrap_or_default()),
                Some(&serde_json::to_value(&updated).unwrap_or_default()),
                None,
                None,
                trace_id,
                "success",
                None,
            )
            .await?;
        Ok(updated)
    }

    pub async fn shutdown(
        &self,
        mac: &str,
        actor: &str,
        trace_id: Option<&str>,
    ) -> Result<NodeRow, AppError> {
        let _ = (actor, trace_id);
        let _node = self
            .repo
            .by_mac(mac)
            .await?
            .ok_or(AppError::NotFound(format!("node {mac}")))?;
        self.repo.set_status(mac, "offline").await?;
        let updated = self
            .repo
            .by_mac(mac)
            .await?
            .ok_or(AppError::NotFound(format!("node {mac}")))?;
        self.hub.publish(Event::NodeStatusChanged {
            id: updated.parse_id().unwrap_or_else(Uuid::nil),
            mac: mac.into(),
            status: "offline".into(),
            at: chrono::Utc::now(),
        });
        Ok(updated)
    }

    pub async fn maintenance(&self, mac: &str) -> Result<NodeRow, AppError> {
        self.repo.set_status(mac, "maintenance").await?;
        self.repo
            .by_mac(mac)
            .await?
            .ok_or(AppError::NotFound(format!("node {mac}")))
    }

    pub async fn reimage(
        &self,
        mac: &str,
        image_id: &Uuid,
        actor: &str,
    ) -> Result<NodeRow, AppError> {
        let _ = image_id;
        self.repo.set_image(mac, Some(&image_id.to_string())).await?;
        self.repo.set_status(mac, "reimaging").await?;
        self.audit
            .record(
                None,
                Some(actor),
                "node.reimage",
                Some("node"),
                Some(mac),
                None,
                Some(&serde_json::json!({ "image_id": image_id })),
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        self.repo
            .by_mac(mac)
            .await?
            .ok_or(AppError::NotFound(format!("node {mac}")))
    }

    pub async fn bulk_wol(&self, req: BulkMacRequest) -> Result<BulkActionResult, AppError> {
        req.validate()?;
        let mut items = Vec::with_capacity(req.macs.len());
        let mut accepted = 0;
        let mut rejected = 0;
        for mac in &req.macs {
            match self.wol.send(mac).await {
                Ok(_) => {
                    items.push(BulkActionItem {
                        mac: mac.clone(),
                        ok: true,
                        error: None,
                    });
                    accepted += 1;
                }
                Err(e) => {
                    items.push(BulkActionItem {
                        mac: mac.clone(),
                        ok: false,
                        error: Some(e.to_string()),
                    });
                    rejected += 1;
                }
            }
        }
        Ok(BulkActionResult {
            accepted,
            rejected,
            details: items,
        })
    }

    pub async fn bulk_shutdown(
        &self,
        req: BulkMacRequest,
        actor: &str,
    ) -> Result<BulkActionResult, AppError> {
        req.validate()?;
        let mut items = Vec::with_capacity(req.macs.len());
        let mut accepted = 0;
        let mut rejected = 0;
        for mac in &req.macs {
            match self.shutdown(mac, actor, None).await {
                Ok(_) => {
                    items.push(BulkActionItem {
                        mac: mac.clone(),
                        ok: true,
                        error: None,
                    });
                    accepted += 1;
                }
                Err(e) => {
                    items.push(BulkActionItem {
                        mac: mac.clone(),
                        ok: false,
                        error: Some(e.to_string()),
                    });
                    rejected += 1;
                }
            }
        }
        Ok(BulkActionResult {
            accepted,
            rejected,
            details: items,
        })
    }

    pub async fn bulk_reimage(
        &self,
        req: BulkReimageRequest,
        actor: &str,
    ) -> Result<BulkActionResult, AppError> {
        req.validate()?;
        let mut items = Vec::with_capacity(req.macs.len());
        let mut accepted = 0;
        let mut rejected = 0;
        for mac in &req.macs {
            match self.reimage(mac, &req.image_id, actor).await {
                Ok(_) => {
                    items.push(BulkActionItem {
                        mac: mac.clone(),
                        ok: true,
                        error: None,
                    });
                    accepted += 1;
                }
                Err(e) => {
                    items.push(BulkActionItem {
                        mac: mac.clone(),
                        ok: false,
                        error: Some(e.to_string()),
                    });
                    rejected += 1;
                }
            }
        }
        Ok(BulkActionResult {
            accepted,
            rejected,
            details: items,
        })
    }
}

// Helper to access id as Uuid.
impl NodeRow {
    pub fn parse_id(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.id).ok()
    }
}
