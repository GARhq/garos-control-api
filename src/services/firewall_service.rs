//! Firewall business logic.

use crate::db::models::firewall_rule::FirewallRuleRow;
use crate::db::repositories::audit::AuditRepo;
use crate::db::repositories::firewall::FirewallRepo;
use crate::domain::firewall::*;
use crate::error::AppError;
use crate::integrations::nftables::{Nftables, NftablesIntegration};
use crate::realtime::events::Event;
use crate::realtime::hub::RealtimeHub;
use validator::Validate;
use std::sync::Arc;
use uuid::Uuid;

pub struct FirewallService {
    repo: FirewallRepo,
    audit: AuditRepo,
    nft: Arc<NftablesIntegration>,
    hub: RealtimeHub,
}

impl FirewallService {
    pub fn new(
        repo: FirewallRepo,
        audit: AuditRepo,
        nft: Arc<NftablesIntegration>,
        hub: RealtimeHub,
    ) -> Self {
        Self {
            repo,
            audit,
            nft,
            hub,
        }
    }

    pub fn repo(&self) -> &FirewallRepo {
        &self.repo
    }

    pub async fn list(&self, enabled_only: bool) -> Result<Vec<FirewallRuleRow>, AppError> {
        self.repo.list(enabled_only).await
    }

    pub async fn list_view(&self) -> Result<Vec<FirewallRuleView>, AppError> {
        let rules = self.repo.list(false).await?;
        Ok(rules.into_iter().map(view_from_row).collect())
    }

    pub async fn by_id(&self, id: &Uuid) -> Result<FirewallRuleRow, AppError> {
        self.repo
            .by_id(id)
            .await?
            .ok_or(AppError::NotFound("firewall rule".into()))
    }

    pub async fn by_id_view(&self, id: &Uuid) -> Result<FirewallRuleView, AppError> {
        let row = self.by_id(id).await?;
        Ok(view_from_row(row))
    }

    pub async fn create(
        &self,
        req: FirewallRuleCreate,
        actor: &str,
    ) -> Result<FirewallRuleView, AppError> {
        req.validate()?;
        let action = req.action.to_lowercase();
        if !["accept", "drop", "reject"].contains(&action.as_str()) {
            return Err(AppError::BadRequest("invalid action".into()));
        }
        let family = req.family.unwrap_or_else(|| "inet".into());
        let chain = req.chain.unwrap_or_else(|| "input".into());
        let priority = req.priority.unwrap_or(0);
        let row = self
            .repo
            .create(
                &action,
                &family,
                "garos",
                &chain,
                req.protocol.as_deref(),
                req.port,
                req.port_end,
                req.source.as_deref(),
                req.destination.as_deref(),
                req.interface_in.as_deref(),
                req.interface_out.as_deref(),
                req.description.as_deref(),
                priority,
                Some(actor),
            )
            .await?;
        let _ = self.nft.add_rule(&row).await?;
        self.audit
            .record(
                None,
                Some(actor),
                "firewall.create",
                Some("firewall_rule"),
                Some(&row.id),
                None,
                Some(&serde_json::to_value(view_from_row(row.clone())).unwrap_or_default()),
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        self.hub.publish(Event::FirewallRuleChanged {
            id: row.id().unwrap_or(Uuid::nil()),
            action: "created".into(),
            at: chrono::Utc::now(),
        });
        Ok(view_from_row(row))
    }

    pub async fn update(
        &self,
        id: &Uuid,
        req: FirewallRuleUpdate,
        actor: &str,
    ) -> Result<FirewallRuleView, AppError> {
        let existing = self.by_id(id).await?;
        let action = req
            .action
            .clone()
            .unwrap_or_else(|| existing.action.clone());
        let row = self
            .repo
            .update(
                id,
                &action,
                req.protocol.as_deref().or(existing.protocol.as_deref()),
                req.port.or(existing.port),
                req.port_end.or(existing.port_end),
                req.source.as_deref().or(existing.source.as_deref()),
                req.destination
                    .as_deref()
                    .or(existing.destination.as_deref()),
                req.description
                    .as_deref()
                    .or(existing.description.as_deref()),
                req.enabled.unwrap_or(existing.enabled),
                req.priority.unwrap_or(existing.priority),
                None,
            )
            .await?;
        self.audit
            .record(
                None,
                Some(actor),
                "firewall.update",
                Some("firewall_rule"),
                Some(&row.id),
                Some(&serde_json::to_value(view_from_row(existing)).unwrap_or_default()),
                Some(&serde_json::to_value(view_from_row(row.clone())).unwrap_or_default()),
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        self.hub.publish(Event::FirewallRuleChanged {
            id: row.id().unwrap_or(Uuid::nil()),
            action: "updated".into(),
            at: chrono::Utc::now(),
        });
        Ok(view_from_row(row))
    }

    pub async fn delete(&self, id: &Uuid, actor: &str) -> Result<(), AppError> {
        let row = self.by_id(id).await?;
        if let Some(handle) = &row.nft_handle {
            self.nft.delete_rule(handle).await.ok();
        }
        self.repo.delete(id).await?;
        let row_id = row.id.clone();
        let before_view = serde_json::to_value(view_from_row(row)).unwrap_or_default();
        self.audit
            .record(
                None,
                Some(actor),
                "firewall.delete",
                Some("firewall_rule"),
                Some(&row_id),
                Some(&before_view),
                None,
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        self.hub.publish(Event::FirewallRuleChanged {
            id: id.clone(),
            action: "deleted".into(),
            at: chrono::Utc::now(),
        });
        Ok(())
    }

    pub async fn preview(&self, req: FirewallRuleCreate) -> Result<FirewallRulePreview, AppError> {
        req.validate()?;
        let row = FirewallRuleRow {
            id: Uuid::now_v7().to_string(),
            action: req.action.clone(),
            family: req.family.clone().unwrap_or_else(|| "inet".into()),
            table_name: "garos".into(),
            chain: req.chain.clone().unwrap_or_else(|| "input".into()),
            protocol: req.protocol.clone(),
            port: req.port,
            port_end: req.port_end,
            source: req.source.clone(),
            destination: req.destination.clone(),
            interface_in: req.interface_in.clone(),
            interface_out: req.interface_out.clone(),
            description: req.description.clone(),
            enabled: req.enabled.unwrap_or(true),
            nft_handle: None,
            priority: req.priority.unwrap_or(0),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: None,
        };
        self.nft.preview_rule(&row).await
    }

    pub async fn panic(&self, activate: bool, actor: &str) -> Result<PanicStatus, AppError> {
        let st = self.nft.panic(activate, Some(actor)).await?;
        self.audit
            .record(
                None,
                Some(actor),
                if activate {
                    "firewall.panic.on"
                } else {
                    "firewall.panic.off"
                },
                Some("firewall"),
                None,
                None,
                Some(&serde_json::to_value(&st).unwrap_or_default()),
                None,
                None,
                None,
                "success",
                None,
            )
            .await?;
        Ok(st)
    }

    pub async fn panic_status(&self) -> Result<PanicStatus, AppError> {
        self.nft.panic_status().await
    }

    pub async fn connections(
        &self,
        limit: usize,
        protocol: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<ConnectionEntry>, AppError> {
        let mut conns = self.nft.list_connections(limit).await?;
        if let Some(p) = protocol {
            conns.retain(|c| c.protocol.eq_ignore_ascii_case(p));
        }
        if let Some(s) = state {
            conns.retain(|c| c.state.eq_ignore_ascii_case(s));
        }
        Ok(conns)
    }

    pub async fn validate(&self) -> Result<Vec<String>, AppError> {
        let rules = self.repo.list(false).await?;
        self.nft.validate(&rules).await
    }
}

pub fn view_from_row(row: FirewallRuleRow) -> FirewallRuleView {
    FirewallRuleView {
        id: Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil()),
        action: row.action,
        family: row.family,
        table_name: row.table_name,
        chain: row.chain,
        protocol: row.protocol,
        port: row.port,
        port_end: row.port_end,
        source: row.source,
        destination: row.destination,
        interface_in: row.interface_in,
        interface_out: row.interface_out,
        description: row.description,
        enabled: row.enabled,
        priority: row.priority,
        nft_handle: row.nft_handle,
    }
}

impl FirewallRuleRow {
    pub fn id(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.id).ok()
    }
}
