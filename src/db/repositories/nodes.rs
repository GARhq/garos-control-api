//! Nodes repository.

use crate::db::models::node::NodeRow;
use crate::db::pool::DbPool;
use crate::error::AppError;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct NodeRepo {
    pool: DbPool,
}

impl NodeRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn by_id(&self, id: &Uuid) -> Result<Option<NodeRow>, AppError> {
        let row = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn by_mac(&self, mac: &str) -> Result<Option<NodeRow>, AppError> {
        let row = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes WHERE mac = ?")
            .bind(mac)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn upsert_heartbeat(
        &self,
        mac: &str,
        ip: Option<&str>,
        hostname: Option<&str>,
        cpu_temp: Option<f64>,
        cpu_usage: Option<f64>,
        mem_usage: Option<f64>,
        ping_ms: Option<f64>,
        nfs_latency: Option<f64>,
        status: Option<&str>,
    ) -> Result<NodeRow, AppError> {
        // Try to find existing.
        if let Some(existing) = self.by_mac(mac).await? {
            let id = existing.id.clone();
            let now = Utc::now();
            let status = status.unwrap_or(&existing.status);
            sqlx::query(
                "UPDATE nodes SET ip = COALESCE(?, ip), hostname = COALESCE(?, hostname),
                  cpu_temp_c = COALESCE(?, cpu_temp_c), cpu_usage_pct = COALESCE(?, cpu_usage_pct),
                  mem_usage_pct = COALESCE(?, mem_usage_pct), ping_ms = COALESCE(?, ping_ms),
                  nfs_latency_ms = COALESCE(?, nfs_latency_ms), last_heartbeat_at = ?,
                  last_seen_at = ?, status = ?, updated_at = ? WHERE id = ?",
            )
            .bind(ip)
            .bind(hostname)
            .bind(cpu_temp)
            .bind(cpu_usage)
            .bind(mem_usage)
            .bind(ping_ms)
            .bind(nfs_latency)
            .bind(now)
            .bind(now)
            .bind(status)
            .bind(now)
            .bind(&id)
            .execute(&self.pool)
            .await?;
            return self
                .by_id(&Uuid::parse_str(&id).unwrap())
                .await?
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("node vanished after update")));
        }
        // Insert new.
        let id = Uuid::now_v7().to_string();
        let now = Utc::now();
        let hostname = hostname.unwrap_or("");
        let status = status.unwrap_or("online");
        sqlx::query(
            "INSERT INTO nodes
               (id, mac, hostname, ip, status, last_heartbeat_at, last_seen_at, cpu_temp_c,
                cpu_usage_pct, mem_usage_pct, ping_ms, nfs_latency_ms, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(mac)
        .bind(hostname)
        .bind(ip)
        .bind(status)
        .bind(now)
        .bind(now)
        .bind(cpu_temp)
        .bind(cpu_usage)
        .bind(mem_usage)
        .bind(ping_ms)
        .bind(nfs_latency)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        self.by_id(&Uuid::parse_str(&id).unwrap())
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("node vanished after insert")))
    }

    pub async fn set_status(&self, mac: &str, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE nodes SET status = ?, updated_at = ? WHERE mac = ?")
            .bind(status)
            .bind(Utc::now())
            .bind(mac)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_image(&self, mac: &str, image_id: Option<&str>) -> Result<(), AppError> {
        sqlx::query("UPDATE nodes SET image_id = ?, updated_at = ? WHERE mac = ?")
            .bind(image_id)
            .bind(Utc::now())
            .bind(mac)
            .execute(&self.pool)
            .await?;
        Ok(())
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
        let sort_col = match sort {
            "hostname" => "hostname",
            "status" => "status",
            "lastSeen" => "last_seen_at",
            "mac" => "mac",
            _ => "mac",
        };
        let order_dir = if order.eq_ignore_ascii_case("desc") {
            "DESC"
        } else {
            "ASC"
        };
        let mut sql = String::from("SELECT * FROM nodes WHERE 1=1");
        let mut binds: Vec<String> = vec![];
        if let Some(s) = status {
            sql.push_str(" AND status = ?");
            binds.push(s.to_string());
        }
        if let Some(i) = image_id {
            sql.push_str(" AND image_id = ?");
            binds.push(i.to_string());
        }
        if let Some(s) = search {
            sql.push_str(" AND (mac LIKE ? OR hostname LIKE ? OR ip LIKE ?)");
            let pat = format!("%{s}%");
            binds.push(pat.clone());
            binds.push(pat.clone());
            binds.push(pat);
        }
        sql.push_str(&format!(" ORDER BY {sort_col} {order_dir} LIMIT ? OFFSET ?"));
        let mut q = sqlx::query_as::<_, NodeRow>(&sql);
        for b in &binds {
            q = q.bind(b);
        }
        q = q.bind(limit).bind(offset);
        Ok(q.fetch_all(&self.pool).await?)
    }

    pub async fn count_for_image(&self, image_id: &str) -> Result<i64, AppError> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM nodes WHERE image_id = ?")
                .bind(image_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }

    pub async fn stats(&self) -> Result<serde_json::Value, AppError> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM nodes")
            .fetch_one(&self.pool)
            .await?;
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*) FROM nodes GROUP BY status",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut by_status = serde_json::Map::new();
        for (k, v) in rows {
            by_status.insert(k, serde_json::json!(v));
        }
        Ok(serde_json::json!({
            "total": total.0,
            "byStatus": by_status,
        }))
    }

    pub async fn delete(&self, mac: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM nodes WHERE mac = ?")
            .bind(mac)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn record_login(
        &self,
        mac: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE nodes SET current_user_id = ?, current_user_role = ?, login_at = ?, updated_at = ? WHERE mac = ?",
        )
        .bind(user_id)
        .bind(role)
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(mac)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
