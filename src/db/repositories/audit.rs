//! Audit log repository.

use crate::db::models::audit_log::AuditLogRow;
use crate::db::pool::DbPool;
use crate::error::AppError;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuditRepo {
    pool: DbPool,
}

impl AuditRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn record(
        &self,
        actor_id: Option<&str>,
        actor_username: Option<&str>,
        action: &str,
        target_type: Option<&str>,
        target_id: Option<&str>,
        before: Option<&Value>,
        after: Option<&Value>,
        ip: Option<&str>,
        user_agent: Option<&str>,
        trace_id: Option<&str>,
        result: &str,
        error_message: Option<&str>,
    ) -> Result<AuditLogRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now();
        let before_json = before.map(|v| serde_json::to_string(v).unwrap_or_default());
        let after_json = after.map(|v| serde_json::to_string(v).unwrap_or_default());
        let before_json_for_struct = before_json.clone();
        let after_json_for_struct = after_json.clone();
        sqlx::query(
            "INSERT INTO audit_log
               (id, actor_id, actor_username, action, target_type, target_id, before_json, after_json,
                ip, user_agent, trace_id, result, error_message, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(actor_id)
        .bind(actor_username)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(before_json)
        .bind(after_json)
        .bind(ip)
        .bind(user_agent)
        .bind(trace_id)
        .bind(result)
        .bind(error_message)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(AuditLogRow {
            id,
            actor_id: actor_id.map(str::to_string),
            actor_username: actor_username.map(str::to_string),
            action: action.to_string(),
            target_type: target_type.map(str::to_string),
            target_id: target_id.map(str::to_string),
            before_json: before_json_for_struct,
            after_json: after_json_for_struct,
            ip: ip.map(str::to_string),
            user_agent: user_agent.map(str::to_string),
            trace_id: trace_id.map(str::to_string),
            result: result.to_string(),
            error_message: error_message.map(str::to_string),
            created_at: now,
        })
    }

    pub async fn list(
        &self,
        actor: Option<&str>,
        action: Option<&str>,
        target_type: Option<&str>,
        target_id: Option<&str>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLogRow>, AppError> {
        let mut sql = String::from("SELECT * FROM audit_log WHERE 1=1");
        let mut binds: Vec<String> = vec![];
        if let Some(a) = actor {
            sql.push_str(" AND actor_username = ?");
            binds.push(a.to_string());
        }
        if let Some(a) = action {
            sql.push_str(" AND action = ?");
            binds.push(a.to_string());
        }
        if let Some(t) = target_type {
            sql.push_str(" AND target_type = ?");
            binds.push(t.to_string());
        }
        if let Some(t) = target_id {
            sql.push_str(" AND target_id = ?");
            binds.push(t.to_string());
        }
        if let Some(f) = from {
            sql.push_str(" AND created_at >= ?");
            binds.push(f.to_rfc3339());
        }
        if let Some(t) = to {
            sql.push_str(" AND created_at <= ?");
            binds.push(t.to_rfc3339());
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");
        let mut q = sqlx::query_as::<_, AuditLogRow>(&sql);
        for b in &binds {
            q = q.bind(b);
        }
        q = q.bind(limit).bind(offset);
        Ok(q.fetch_all(&self.pool).await?)
    }

    pub async fn by_id(&self, id: &str) -> Result<Option<AuditLogRow>, AppError> {
        let row = sqlx::query_as::<_, AuditLogRow>("SELECT * FROM audit_log WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn stats(&self) -> Result<Value, AppError> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_log")
            .fetch_one(&self.pool)
            .await?;
        let by_action: Vec<(String, i64)> = sqlx::query_as(
            "SELECT action, COUNT(*) FROM audit_log GROUP BY action ORDER BY 2 DESC LIMIT 25",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut map = serde_json::Map::new();
        for (k, v) in by_action {
            map.insert(k, serde_json::json!(v));
        }
        Ok(serde_json::json!({
            "total": total.0,
            "byAction": map,
        }))
    }
}
