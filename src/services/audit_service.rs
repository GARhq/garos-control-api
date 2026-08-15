//! Audit service.

use crate::db::models::audit_log::AuditLogRow;
use crate::db::repositories::audit::AuditRepo;
use crate::domain::audit::*;
use crate::error::AppError;
use validator::Validate;
use chrono::{DateTime, Utc};
use std::io::Write;

pub struct AuditService {
    repo: AuditRepo,
}

impl AuditService {
    pub fn new(repo: AuditRepo) -> Self {
        Self { repo }
    }

    pub fn repo(&self) -> &AuditRepo {
        &self.repo
    }

    pub async fn list(
        &self,
        q: AuditQuery,
    ) -> Result<Vec<AuditEntry>, AppError> {
        q.validate()?;
        let limit = q.limit.unwrap_or(50);
        let offset = q.offset.unwrap_or(0);
        let rows = self
            .repo
            .list(
                q.actor.as_deref(),
                q.action.as_deref(),
                None,
                q.target.as_deref(),
                q.from,
                q.to,
                limit,
                offset,
            )
            .await?;
        Ok(rows.into_iter().map(entry_from_row).collect())
    }

    pub async fn by_id(&self, id: &str) -> Result<AuditEntry, AppError> {
        let row = self
            .repo
            .by_id(id)
            .await?
            .ok_or(AppError::NotFound("audit entry".into()))?;
        Ok(entry_from_row(row))
    }

    pub async fn stats(&self) -> Result<AuditStats, AppError> {
        let s = self.repo.stats().await?;
        let by_action = s["byAction"]
            .as_object()
            .map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_i64().unwrap_or(0)))
                    .collect()
            })
            .unwrap_or_default();
        Ok(AuditStats {
            total: s["total"].as_i64().unwrap_or(0),
            by_action,
        })
    }

    pub async fn export(
        &self,
        format: &str,
        q: AuditQuery,
    ) -> Result<Vec<u8>, AppError> {
        let entries = self.list(q).await?;
        match format {
            "json" => Ok(serde_json::to_vec_pretty(&entries)?),
            "cef" => Ok(entries
                .iter()
                .map(cef_encode)
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes()),
            "leef" => Ok(entries
                .iter()
                .map(leef_encode)
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes()),
            other => Err(AppError::BadRequest(format!("unknown format: {other}"))),
        }
    }
}

pub fn entry_from_row(row: AuditLogRow) -> AuditEntry {
    AuditEntry {
        id: uuid::Uuid::parse_str(&row.id).unwrap_or_else(|_| uuid::Uuid::nil()),
        actor_id: row
            .actor_id
            .as_deref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok()),
        actor_username: row.actor_username,
        action: row.action,
        target_type: row.target_type,
        target_id: row.target_id,
        before: row.before_json.and_then(|s| serde_json::from_str(&s).ok()),
        after: row.after_json.and_then(|s| serde_json::from_str(&s).ok()),
        ip: row.ip,
        user_agent: row.user_agent,
        trace_id: row.trace_id,
        result: row.result,
        error_message: row.error_message,
        created_at: row.created_at,
    }
}

fn cef_encode(e: &AuditEntry) -> String {
    // CEF:Version|Device Vendor|Device Product|Device Version|Signature ID|Name|Severity|Extension
    let sev = match e.result.as_str() {
        "failure" | "denied" => 7,
        _ => 1,
    };
    let ext = format!(
        "act={} suser={} target={} rt={} cs1Label=traceId cs1={}",
        e.action,
        e.actor_username.clone().unwrap_or_default(),
        e.target_id.clone().unwrap_or_default(),
        e.created_at.to_rfc3339(),
        e.trace_id.clone().unwrap_or_default(),
    );
    format!(
        "CEF:0|kryonix|garos|1|{}|{}|{}|{}",
        e.action, e.action, sev, ext
    )
}

fn leef_encode(e: &AuditEntry) -> String {
    // LEEF:Version|Vendor|Product|Version|EventID|...attributes
    let mut s = format!(
        "LEEF:1.0|kryonix|garos|1.0|{}|",
        e.action.replace('\t', " ")
    );
    s.push_str(&format!(
        "devTime={}\tuserName={}\tsrc={}\tseverity={}\n",
        e.created_at.to_rfc3339(),
        e.actor_username.clone().unwrap_or_default(),
        e.ip.clone().unwrap_or_default(),
        if e.result == "success" { 1 } else { 7 }
    ));
    s
}

#[allow(dead_code)]
fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
