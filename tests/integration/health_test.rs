//! Health integration test.

use crate::common::TestApp;

#[tokio::test]
async fn health_db_works() {
    let app = TestApp::new().await;
    let row: (i64,) = sqlx::query_as("SELECT 1")
        .fetch_one(&app.pool)
        .await
        .expect("select 1");
    assert_eq!(row.0, 1);
}
