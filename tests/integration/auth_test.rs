//! Auth integration tests.

use crate::common::TestApp;

#[tokio::test]
async fn admin_can_log_in() {
    let app = TestApp::new().await;
    app.seed_admin().await;
    let resp = app
        .state
        .users
        .login(
            garos_backend::domain::user::LoginRequest {
                username: "admin".into(),
                password: "ChangeMe!2024".into(),
            },
            Some("127.0.0.1"),
            Some("test"),
        )
        .await
        .expect("login");
    assert!(!resp.access_token.is_empty());
}

#[tokio::test]
async fn wrong_password_is_unauthorized() {
    let app = TestApp::new().await;
    app.seed_admin().await;
    let r = app
        .state
        .users
        .login(
            garos_backend::domain::user::LoginRequest {
                username: "admin".into(),
                password: "nope".into(),
            },
            None,
            None,
        )
        .await;
    assert!(r.is_err());
}
