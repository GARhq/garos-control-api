//! Prometheus metrics.

use prometheus::{
    register_counter_vec_with_registry, register_gauge_vec_with_registry,
    register_histogram_vec_with_registry, register_int_counter_with_registry,
    register_int_gauge_with_registry, CounterVec, Encoder, GaugeVec, HistogramVec, IntCounter,
    IntGauge, Registry, TextEncoder,
};

#[cfg(target_os = "linux")]
use prometheus::process_collector::ProcessCollector;

pub struct Metrics {
    pub registry: Registry,
    pub http_requests_total: CounterVec,
    pub http_request_duration: HistogramVec,
    pub active_connections: IntGauge,
    pub node_heartbeats_total: CounterVec,
    pub image_builds_total: CounterVec,
    pub firewall_rules_count: IntGauge,
    pub audit_log_entries_total: IntCounter,
    pub db_pool_size: GaugeVec,
    pub integration_errors_total: CounterVec,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let http_requests_total = register_counter_vec_with_registry!(
            "garos_http_requests_total",
            "Total HTTP requests by method/route/status",
            &["method", "route", "status"],
            registry
        )?;
        let http_request_duration = register_histogram_vec_with_registry!(
            "garos_http_request_duration_seconds",
            "HTTP request duration in seconds",
            &["method", "route"],
            vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
            registry
        )?;
        let active_connections = register_int_gauge_with_registry!(
            "garos_active_connections",
            "Active connections",
            registry
        )?;
        let node_heartbeats_total = register_counter_vec_with_registry!(
            "garos_node_heartbeats_total",
            "Node heartbeats received",
            &["status"],
            registry
        )?;
        let image_builds_total = register_counter_vec_with_registry!(
            "garos_image_builds_total",
            "Image builds by status",
            &["status"],
            registry
        )?;
        let firewall_rules_count = register_int_gauge_with_registry!(
            "garos_firewall_rules_count",
            "Active firewall rules",
            registry
        )?;
        let audit_log_entries_total = register_int_counter_with_registry!(
            "garos_audit_log_entries_total",
            "Total audit log entries",
            registry
        )?;
        let db_pool_size = register_gauge_vec_with_registry!(
            "garos_db_pool_size",
            "DB pool size",
            &["state"],
            registry
        )?;
        let integration_errors_total = register_counter_vec_with_registry!(
            "garos_integration_errors_total",
            "Integration errors by kind",
            &["kind"],
            registry
        )?;

        #[cfg(target_os = "linux")]
        registry.register(Box::new(ProcessCollector::for_self()))?;

        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration,
            active_connections,
            node_heartbeats_total,
            image_builds_total,
            firewall_rules_count,
            audit_log_entries_total,
            db_pool_size,
            integration_errors_total,
        })
    }

    pub fn render(&self) -> Result<String, prometheus::Error> {
        let mut buf = Vec::new();
        let encoder = TextEncoder::new();
        let metrics = self.registry.gather();
        encoder.encode(&metrics, &mut buf)?;
        Ok(String::from_utf8(buf).unwrap_or_default())
    }
}
