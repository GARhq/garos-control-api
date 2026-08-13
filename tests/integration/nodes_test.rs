//! Node integration tests.

use crate::common::TestApp;
use garos_backend::domain::node::{BulkMacRequest, HeartbeatRequest};

#[tokio::test]
async fn heartbeat_creates_node() {
    let app = TestApp::new().await;
    let row = app
        .state
        .nodes
        .heartbeat(
            "AA:BB:CC:DD:EE:FF",
            HeartbeatRequest {
                hostname: Some("test-01".into()),
                ip: Some("10.0.0.10".into()),
                cpu_temp_c: Some(45.0),
                cpu_usage_pct: Some(15.0),
                mem_usage_pct: Some(50.0),
                ping_ms: Some(2.0),
                nfs_latency_ms: Some(1.0),
                status: Some("online".into()),
            },
        )
        .await
        .expect("heartbeat");
    assert_eq!(row.mac, "AA:BB:CC:DD:EE:FF");
}

#[tokio::test]
async fn bulk_wol_reports_each() {
    let app = TestApp::new().await;
    let r = app
        .state
        .nodes
        .bulk_wol(BulkMacRequest {
            macs: vec![
                "AA:BB:CC:DD:EE:FF".into(),
                "11:22:33:44:55:66".into(),
            ],
        })
        .await
        .expect("bulk wol");
    assert_eq!(r.accepted, 2);
    assert_eq!(r.rejected, 0);
}
