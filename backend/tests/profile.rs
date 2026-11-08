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

#[tokio::test]
async fn changing_the_email_requires_the_current_password() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let missing = browser
        .patch_json("/profile", &json!({ "email": "ana.new@example.com" }))
        .await;
    assert_eq!(missing.status(), 422);
    let body: Value = missing.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("validation_failed"));
    assert_eq!(
        body["error"]["fields"]["current_password"],
        json!("required")
    );

    let wrong = browser
        .patch_json(
            "/profile",
            &json!({ "email": "ana.new@example.com", "current_password": "not my password" }),
        )
        .await;
    assert_eq!(wrong.status(), 403);
    let body: Value = wrong.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("invalid_current_password"));

    let unchanged: Value = browser.get("/auth/me").await.json().await.unwrap();
    assert_eq!(unchanged["user"]["email"], json!("ana@example.com"));
}

#[tokio::test]
async fn with_the_current_password_the_email_changes_and_the_new_one_logs_in() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let response = browser
        .patch_json(
            "/profile",
            &json!({ "email": " Ana.New@Example.com ", "current_password": PASSWORD }),
        )
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["user"]["email"], json!("Ana.New@Example.com"));
    let other_browser = app.session();
    assert_eq!(
        other_browser
            .log_in("ana@example.com", PASSWORD)
            .await
            .status(),
        401
    );
    assert_eq!(
        other_browser
            .log_in("ana.new@example.com", PASSWORD)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn sending_the_unchanged_email_with_a_new_name_needs_no_password() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let response = browser
        .patch_json(
            "/profile",
            &json!({ "name": "Ana", "email": "ana@example.com" }),
        )
        .await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn changing_the_password_requires_the_current_one_and_the_new_one_replaces_the_old() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let new_password = "a brand new passphrase";

    let missing = browser
        .patch_json("/profile", &json!({ "new_password": new_password }))
        .await;
    assert_eq!(missing.status(), 422);

    let wrong = browser
        .patch_json(
            "/profile",
            &json!({ "new_password": new_password, "current_password": "not my password" }),
        )
        .await;
    assert_eq!(wrong.status(), 403);
    assert_eq!(
        app.session()
            .log_in("ana@example.com", PASSWORD)
            .await
            .status(),
        200,
        "a refused change must keep the old password"
    );

    let short = browser
        .patch_json(
            "/profile",
            &json!({ "new_password": "short", "current_password": PASSWORD }),
        )
        .await;
    assert_eq!(short.status(), 422);
    let body: Value = short.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["new_password"],
        json!("password_length")
    );

    let changed = browser
        .patch_json(
            "/profile",
            &json!({ "new_password": new_password, "current_password": PASSWORD }),
        )
        .await;
    assert_eq!(changed.status(), 200);
    let other_browser = app.session();
    assert_eq!(
        other_browser
            .log_in("ana@example.com", PASSWORD)
            .await
            .status(),
        401
    );
    assert_eq!(
        other_browser
            .log_in("ana@example.com", new_password)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn the_profile_can_only_be_edited_while_signed_in_and_only_for_oneself() {
    let app = TestApp::spawn().await;
    let visitor = app.session();
    let response = visitor
        .patch_json("/profile", &json!({ "name": "Nobody" }))
        .await;
    assert_eq!(response.status(), 401);

    app.session().sign_up("bia@example.com", PASSWORD).await;
    let ana = app.session();
    ana.sign_up("ana@example.com", PASSWORD).await;
    ana.patch_json("/profile", &json!({ "name": "Ana" })).await;

    let bia = app.session();
    bia.log_in("bia@example.com", PASSWORD).await;
    let me: Value = bia.get("/auth/me").await.json().await.unwrap();
    assert_eq!(me["user"]["name"], json!(""));
}

#[tokio::test]
async fn the_email_cannot_be_taken_from_another_account_nor_be_invalid() {
    let app = TestApp::spawn().await;
    app.session().sign_up("bia@example.com", PASSWORD).await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let taken = browser
        .patch_json(
            "/profile",
            &json!({ "email": "BIA@example.com", "current_password": PASSWORD }),
        )
        .await;
    assert_eq!(taken.status(), 409);
    let body: Value = taken.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("email_taken"));

    let invalid = browser
        .patch_json(
            "/profile",
            &json!({ "email": "nope", "current_password": PASSWORD }),
        )
        .await;
    assert_eq!(invalid.status(), 422);
    let body: Value = invalid.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["email"], json!("invalid_email"));
}
