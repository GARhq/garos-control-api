//! Realtime pub/sub hub.

use crate::realtime::events::{Channel, Event};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

pub type ClientId = Uuid;

/// Receiver side of a subscription.
pub type Subscriber = broadcast::Receiver<Arc<Event>>;

/// Hub: produces events, fans them out to subscribers, with a per-channel
/// fan-out that filters messages that don't match the subscription.
#[derive(Debug, Clone)]
pub struct RealtimeHub {
    inner: Arc<HubInner>,
}

#[derive(Debug)]
struct HubInner {
    /// Global broadcast — every event flows here.
    broadcast: broadcast::Sender<Arc<Event>>,
    /// Counter for unique IDs and metrics.
    counter: AtomicU64,
    /// Active subscribers.
    clients: RwLock<HashMap<ClientId, ClientState>>,
}

#[derive(Debug, Clone)]
struct ClientState {
    channels: Vec<Channel>,
}

impl RealtimeHub {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self {
            inner: Arc::new(HubInner {
                broadcast: tx,
                counter: AtomicU64::new(0),
                clients: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Publish an event to all subscribers.
    pub fn publish(&self, ev: Event) {
        let arc = Arc::new(ev.clone());
        // The global broadcast sees the event; subscribers filter.
        let _ = self.inner.broadcast.send(arc);
        // Track stats in tracing.
        tracing::debug!(target: "realtime", ev = %ev_kind(&ev), "published");
    }

    /// Subscribe to a set of channels.
    pub fn subscribe(&self, channels: Vec<Channel>) -> (ClientId, Subscriber) {
        let id = Uuid::now_v7();
        let rx = self.inner.broadcast.subscribe();
        self.inner.clients.write().insert(
            id,
            ClientState {
                channels: if channels.is_empty() {
                    vec![Channel::All]
                } else {
                    channels
                },
            },
        );
        self.inner.counter.fetch_add(1, Ordering::Relaxed);
        (id, rx)
    }

    pub fn unsubscribe(&self, id: ClientId) {
        self.inner.clients.write().remove(&id);
    }

    /// Filter: true if the event should be delivered to this client.
    pub fn matches(id: ClientId, _ev: &Event) -> bool {
        // We can't read the client's channel list without a callback into
        // the hub, so this is a no-op at the hub level — clients filter
        // after receiving by inspecting `Event::channel()`.
        let _ = id;
        true
    }

    pub fn active_clients(&self) -> usize {
        self.inner.clients.read().len()
    }

    pub fn event_count(&self) -> u64 {
        self.inner.counter.load(Ordering::Relaxed)
    }
}

impl Default for RealtimeHub {
    fn default() -> Self {
        Self::new(1024)
    }
}

fn ev_kind(ev: &Event) -> &'static str {
    match ev {
        Event::NodeStatusChanged { .. } => "node_status_changed",
        Event::NodeHeartbeat { .. } => "node_heartbeat",
        Event::MetricsUpdate { .. } => "metrics_update",
        Event::AuditLogAdded { .. } => "audit_log_added",
        Event::ServiceStatusChanged { .. } => "service_status_changed",
        Event::ImageBuildProgress { .. } => "image_build_progress",
        Event::FirewallRuleChanged { .. } => "firewall_rule_changed",
        Event::Ping { .. } => "ping",
        Event::Pong { .. } => "pong",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_and_receive() {
        let hub = RealtimeHub::new(8);
        let (_id, mut rx) = hub.subscribe(vec![Channel::All]);
        hub.publish(Event::Ping { at: chrono::Utc::now() });
        let got = rx.recv().await.unwrap();
        assert!(matches!(*got, Event::Ping { .. }));
    }
}
