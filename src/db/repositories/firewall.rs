//! Firewall rules repository.

use crate::db::models::firewall_rule::FirewallRuleRow;
use crate::db::pool::DbPool;
use crate::error::AppError;
use uuid::Uuid;

#[derive(Clone)]
pub struct FirewallRepo {
    pool: DbPool,
}

impl FirewallRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, enabled_only: bool) -> Result<Vec<FirewallRuleRow>, AppError> {
        let sql = if enabled_only {
            "SELECT * FROM firewall_rules WHERE enabled = 1 ORDER BY priority ASC, created_at ASC"
        } else {
            "SELECT * FROM firewall_rules ORDER BY priority ASC, created_at ASC"
        };
        let rows = sqlx::query_as::<_, FirewallRuleRow>(sql)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn by_id(&self, id: &Uuid) -> Result<Option<FirewallRuleRow>, AppError> {
        let row = sqlx::query_as::<_, FirewallRuleRow>("SELECT * FROM firewall_rules WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn create(
        &self,
        action: &str,
        family: &str,
        table: &str,
        chain: &str,
        protocol: Option<&str>,
        port: Option<i32>,
        port_end: Option<i32>,
        source: Option<&str>,
        destination: Option<&str>,
        interface_in: Option<&str>,
        interface_out: Option<&str>,
        description: Option<&str>,
        priority: i32,
        created_by: Option<&str>,
    ) -> Result<FirewallRuleRow, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO firewall_rules
               (id, action, family, table_name, chain, protocol, port, port_end, source, destination,
                interface_in, interface_out, description, enabled, priority, created_at, updated_at, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(action)
        .bind(family)
        .bind(table)
        .bind(chain)
        .bind(protocol)
        .bind(port)
        .bind(port_end)
        .bind(source)
        .bind(destination)
        .bind(interface_in)
        .bind(interface_out)
        .bind(description)
        .bind(priority)
        .bind(now)
        .bind(now)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        self.by_id(&Uuid::parse_str(&id).unwrap())
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("fw rule vanished")))
    }

    pub async fn update(
        &self,
        id: &Uuid,
        action: &str,
        protocol: Option<&str>,
        port: Option<i32>,
        port_end: Option<i32>,
        source: Option<&str>,
        destination: Option<&str>,
        description: Option<&str>,
        enabled: bool,
        priority: i32,
        nft_handle: Option<&str>,
    ) -> Result<FirewallRuleRow, AppError> {
        sqlx::query(
            "UPDATE firewall_rules SET action = ?, protocol = ?, port = ?, port_end = ?, source = ?, destination = ?,
             description = ?, enabled = ?, priority = ?, nft_handle = COALESCE(?, nft_handle), updated_at = ? WHERE id = ?",
        )
        .bind(action)
        .bind(protocol)
        .bind(port)
        .bind(port_end)
        .bind(source)
        .bind(destination)
        .bind(description)
        .bind(enabled)
        .bind(priority)
        .bind(nft_handle)
        .bind(chrono::Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        self.by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("firewall rule".into()))
    }

    pub async fn delete(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM firewall_rules WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn count_enabled(&self) -> Result<i64, AppError> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM firewall_rules WHERE enabled = 1")
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }
}
