mod common;

use common::TestApp;
use serde_json::{Value, json};

#[tokio::test]
async fn health_reports_ok_when_database_is_reachable() {
    let app = TestApp::spawn().await;

    let response = app.get("/health").await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["status"], json!("ok"));
    assert_eq!(body["database"], json!("up"));
}
