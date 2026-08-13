//! Firewall integration tests.

use crate::common::TestApp;
use garos_backend::domain::firewall::FirewallRuleCreate;

#[tokio::test]
async fn create_and_validate() {
    let app = TestApp::new().await;
    let v = app
        .state
        .firewall
        .create(
            FirewallRuleCreate {
                action: "accept".into(),
                family: None,
                chain: None,
                protocol: Some("tcp".into()),
                port: Some(443),
                port_end: None,
                source: None,
                destination: None,
                interface_in: None,
                interface_out: None,
                description: Some("HTTPS in".into()),
                enabled: Some(true),
                priority: Some(0),
            },
            "test",
        )
        .await
        .expect("create rule");
    assert!(v.enabled);
    let conflicts = app.state.firewall.validate().await.expect("validate");
    // No conflicts with a single rule.
    assert!(conflicts.is_empty());
}
