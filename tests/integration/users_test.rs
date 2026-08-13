//! User integration tests.

use crate::common::TestApp;
use garos_backend::domain::user::UserCreate;

#[tokio::test]
async fn create_user_and_find() {
    let app = TestApp::new().await;
    let u = app
        .state
        .users
        .create(UserCreate {
            username: "alice".into(),
            email: Some("alice@test.local".into()),
            display_name: Some("Alice".into()),
            password: "Hunter22x!".into(),
            role: "operator".into(),
            samba_dn: None,
        })
        .await
        .expect("create");
    let found = app.state.users.by_id(&u.id()).await.expect("by_id");
    assert!(found.is_some());
}
