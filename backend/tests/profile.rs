//! Editing the profile (UC14): name, e-mail and password, through the HTTP API.

mod common;

use common::TestApp;
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";

#[tokio::test]
async fn a_new_account_has_the_name_from_the_sign_up_and_the_user_can_change_it_without_the_password()
 {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let before: Value = browser.get("/auth/me").await.json().await.unwrap();
    assert_eq!(before["user"]["name"], json!(common::DEFAULT_NAME));

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

// Asking for a new e-mail only creates a pending change; the whole flow
// (confirmation, notice, expiry, limits) is in `tests/email_change.rs`.

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
        json!("password_too_short")
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
    assert_eq!(me["user"]["name"], json!(common::DEFAULT_NAME));
}

/// Asks for each `(new_password, code)` in the profile of a signed-in account
/// and checks that the stable code comes back under `new_password`, and that
/// the old password keeps working.
async fn assert_profile_refuses(app: &TestApp, email: &str, cases: &[(&str, &str)]) {
    let browser = app.session();
    browser.log_in(email, PASSWORD).await;

    for (new_password, code) in cases {
        let response = browser
            .patch_json(
                "/profile",
                &json!({ "new_password": new_password, "current_password": PASSWORD }),
            )
            .await;
        assert_eq!(response.status(), 422, "{new_password:?} should be refused");
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], json!("validation_failed"));
        assert_eq!(
            body["error"]["fields"]["new_password"],
            json!(code),
            "wrong code for {new_password:?}"
        );
    }

    assert_eq!(
        app.session().log_in(email, PASSWORD).await.status(),
        200,
        "refused changes must keep the old password"
    );
}

#[tokio::test]
async fn the_profile_refuses_short_long_common_and_sequence_passwords_with_a_code_for_each() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let too_long = "the quick brown fox jumps over the lazy dog ".repeat(3);

    assert_profile_refuses(
        &app,
        "ana@example.com",
        &[
            ("12345678", "password_too_short"),
            ("password", "password_too_short"),
            ("senha123", "password_too_short"),
            (&too_long, "password_too_long"),
            ("aaaaaaaaaa", "password_sequence"),
            ("1234567890", "password_sequence"),
            ("abcdefghij", "password_sequence"),
            ("1234512345", "password_sequence"),
            ("PASSWORD123", "password_common"),
            ("Senha123456!", "password_common"),
            ("Brasil12345", "password_common"),
            ("qwerty123456", "password_common"),
        ],
    )
    .await;
}

#[tokio::test]
async fn the_profile_refuses_a_password_with_the_email_or_the_name_of_the_own_account() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("bruno.lima@example.com", PASSWORD).await;
    browser
        .patch_json("/profile", &json!({ "name": "Maria José Aparecida" }))
        .await;

    assert_profile_refuses(
        &app,
        "bruno.lima@example.com",
        &[
            ("xk!bruno.lima#92", "password_contains_email"),
            ("BRUNOLIMA is here", "password_contains_email"),
            ("my password is aparecida", "password_contains_name"),
            ("trust-JOSE-with-everything", "password_contains_name"),
            ("maria jose aparecida", "password_contains_name"),
        ],
    )
    .await;
}

#[tokio::test]
async fn the_profile_judges_the_password_against_its_own_account_only() {
    let app = TestApp::spawn().await;
    app.session()
        .sign_up("beatriz.souza@example.com", PASSWORD)
        .await;
    let ana = app.session();
    ana.sign_up("ana@example.com", PASSWORD).await;

    // Another account's e-mail is just text for this one.
    let response = ana
        .patch_json(
            "/profile",
            &json!({
                "new_password": "xk!beatriz.souza#92 loves tea",
                "current_password": PASSWORD
            }),
        )
        .await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn the_profile_ignores_short_name_fragments_and_accepts_a_long_passphrase() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .patch_json("/profile", &json!({ "name": "Ana Li" }))
        .await;
    let passphrase = "li and ana walk to the old market every morning";

    let response = browser
        .patch_json(
            "/profile",
            &json!({ "new_password": passphrase, "current_password": PASSWORD }),
        )
        .await;

    assert_eq!(response.status(), 200);
    assert_eq!(
        app.session()
            .log_in("ana@example.com", passphrase)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn a_weak_password_is_refused_before_the_current_password_is_checked() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let response = browser
        .patch_json(
            "/profile",
            &json!({ "new_password": "1234567890", "current_password": "not my password" }),
        )
        .await;

    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["new_password"],
        json!("password_sequence")
    );
}
