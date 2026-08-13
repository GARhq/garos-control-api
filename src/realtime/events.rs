//! Realtime event types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Channel a client can subscribe to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// All events.
    All,
    /// `NodeStatusChanged`, `NodeHeartbeat`, `MetricsUpdate`.
    Nodes,
    /// `MetricsUpdate`.
    Metrics,
    /// `AuditLogAdded`.
    Audit,
    /// `ServiceStatusChanged`.
    Services,
    /// `ImageBuildProgress`.
    ImageBuilds,
    /// `FirewallRuleChanged`.
    Firewall,
}

impl Channel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Self::All),
            "nodes" => Some(Self::Nodes),
            "metrics" => Some(Self::Metrics),
            "audit" => Some(Self::Audit),
            "services" => Some(Self::Services),
            "image_builds" | "images" => Some(Self::ImageBuilds),
            "firewall" => Some(Self::Firewall),
            _ => None,
        }
    }
}

/// All events that can flow through the realtime hub.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    NodeStatusChanged {
        id: Uuid,
        mac: String,
        status: String,
        at: DateTime<Utc>,
    },
    NodeHeartbeat {
        mac: String,
        cpu_temp_c: Option<f64>,
        cpu_usage_pct: Option<f64>,
        mem_usage_pct: Option<f64>,
        at: DateTime<Utc>,
    },
    MetricsUpdate {
        cpu_pct: f64,
        mem_pct: f64,
        disk_pct: f64,
        at: DateTime<Utc>,
    },
    AuditLogAdded {
        id: Uuid,
        action: String,
        actor: Option<String>,
        target: Option<String>,
        at: DateTime<Utc>,
    },
    ServiceStatusChanged {
        name: String,
        state: String,
        needs_attention: bool,
        at: DateTime<Utc>,
    },
    ImageBuildProgress {
        image_id: Uuid,
        status: String,
        progress_pct: f32,
        at: DateTime<Utc>,
    },
    FirewallRuleChanged {
        id: Uuid,
        action: String,
        at: DateTime<Utc>,
    },
    Ping {
        at: DateTime<Utc>,
    },
    Pong {
        at: DateTime<Utc>,
    },
}

impl Event {
    /// Which channel an event belongs to.
    pub fn channel(&self) -> Channel {
        match self {
            Self::NodeStatusChanged { .. } | Self::NodeHeartbeat { .. } => Channel::Nodes,
            Self::MetricsUpdate { .. } => Channel::Metrics,
            Self::AuditLogAdded { .. } => Channel::Audit,
            Self::ServiceStatusChanged { .. } => Channel::Services,
            Self::ImageBuildProgress { .. } => Channel::ImageBuilds,
            Self::FirewallRuleChanged { .. } => Channel::Firewall,
            Self::Ping { .. } | Self::Pong { .. } => Channel::All,
        }
    }
}
