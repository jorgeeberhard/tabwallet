//! Editing the profile (UC14): name, e-mail and password, through the HTTP API.

mod common;

use common::TestApp;
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";

#[tokio::test]
async fn a_new_account_has_no_name_and_the_user_can_set_one_without_the_password() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let before: Value = browser.get("/auth/me").await.json().await.unwrap();
    assert_eq!(before["user"]["name"], json!(""));

    let response = browser
        .patch_json("/profile", &json!({ "name": "  Ana Souza  " }))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["user"]["name"], json!("Ana Souza"));
    let after: Value = browser.get("/auth/me").await.json().await.unwrap();
    assert_eq!(after["user"]["name"], json!("Ana Souza"));
    assert_eq!(after["user"]["email"], json!("ana@example.com"));
}
