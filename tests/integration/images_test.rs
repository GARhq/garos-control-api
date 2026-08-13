//! Image integration tests.

use crate::common::TestApp;
use garos_backend::domain::image::ImageCreate;
use uuid::Uuid;

#[tokio::test]
async fn create_and_publish() {
    let app = TestApp::new().await;
    let author = Uuid::now_v7();
    let img = app
        .state
        .images
        .create(
            ImageCreate {
                name: "test-image".into(),
                description: Some("desc".into()),
                nixos_version: Some("24.05".into()),
                kernel: Some("bzImage".into()),
                kernel_args: Some("quiet".into()),
                packages: vec!["curl".into(), "jq".into()],
                custom_nix: None,
                version: "1.0.0".into(),
            },
            &author,
        )
        .await
        .expect("create");
    app.state
        .images
        .publish(&img.id(), "test")
        .await
        .expect("publish");
}
