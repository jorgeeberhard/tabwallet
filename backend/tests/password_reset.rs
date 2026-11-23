//! Password recovery by e-mail (UC03), through the HTTP API with a recording
//! e-mail sender in place of a real provider.

mod common;

use chrono::Duration;
use common::{Session, TEST_APP_URL, TestApp};
use serde_json::{Value, json};
use tabwallet_backend::email::EmailMessage;

const PASSWORD: &str = "correct horse battery";
const NEW_PASSWORD: &str = "a brand new passphrase";

/// The reset token inside the link of an e-mail.
fn token_in(email: &EmailMessage) -> String {
    let link_start = format!("{TEST_APP_URL}/reset-password?token=");
    let rest = email
        .text
        .split_once(&link_start)
        .unwrap_or_else(|| panic!("no reset link in the e-mail: {}", email.text))
        .1;
    rest.split(char::is_whitespace).next().unwrap().to_string()
}

async fn request_reset(session: &Session, email: &str) -> reqwest::Response {
    session
        .post_json("/auth/password-reset/request", &json!({ "email": email }))
        .await
}

async fn confirm_reset(session: &Session, token: &str, password: &str) -> reqwest::Response {
    session
        .post_json(
            "/auth/password-reset/confirm",
            &json!({ "token": token, "password": password }),
        )
        .await
}

#[tokio::test]
async fn the_reset_link_goes_to_the_login_email_and_an_unknown_email_gets_the_same_answer() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let visitor = app.session();

    let known = request_reset(&visitor, "ANA@example.com").await;
    let known_status = known.status();
    let known_body = known.text().await.unwrap();
    let unknown = request_reset(&visitor, "nobody@example.com").await;
    let unknown_status = unknown.status();
    let unknown_body = unknown.text().await.unwrap();

    assert_eq!(known_status, 202);
    assert_eq!((known_status, known_body), (unknown_status, unknown_body));
    let sent = app.sent_emails();
    assert_eq!(sent.len(), 1, "only the registered address gets an e-mail");
    assert_eq!(sent[0].to, "ana@example.com");
    assert!(!token_in(&sent[0]).is_empty());
}

#[tokio::test]
async fn an_invalid_email_is_rejected_without_sending_anything() {
    let app = TestApp::spawn().await;

    let response = request_reset(&app.session(), "not-an-email").await;

    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["email"], json!("invalid_email"));
    assert!(app.sent_emails().is_empty());
}

#[tokio::test]
async fn the_link_sets_a_new_password_and_the_old_one_stops_working() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let visitor = app.session();
    request_reset(&visitor, "ana@example.com").await;
    let token = token_in(&app.sent_emails()[0]);

    let response = confirm_reset(&visitor, &token, NEW_PASSWORD).await;

    assert_eq!(response.status(), 204);
    assert_eq!(
        visitor.log_in("ana@example.com", PASSWORD).await.status(),
        401
    );
    assert_eq!(
        visitor
            .log_in("ana@example.com", NEW_PASSWORD)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn a_link_that_was_already_used_is_rejected() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let visitor = app.session();
    request_reset(&visitor, "ana@example.com").await;
    let token = token_in(&app.sent_emails()[0]);
    confirm_reset(&visitor, &token, NEW_PASSWORD).await;

    let again = confirm_reset(&visitor, &token, "yet another passphrase").await;

    assert_eq!(again.status(), 400);
    let body: Value = again.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("invalid_reset_token"));
    assert_eq!(
        visitor
            .log_in("ana@example.com", NEW_PASSWORD)
            .await
            .status(),
        200,
        "the second attempt must not change the password"
    );
}

#[tokio::test]
async fn a_link_stops_working_after_an_hour() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let visitor = app.session();
    request_reset(&visitor, "ana@example.com").await;
    let token = token_in(&app.sent_emails()[0]);

    app.advance_time(Duration::minutes(61));
    let late = confirm_reset(&visitor, &token, NEW_PASSWORD).await;

    assert_eq!(late.status(), 400);
    let body: Value = late.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("invalid_reset_token"));
    assert_eq!(
        visitor.log_in("ana@example.com", PASSWORD).await.status(),
        200
    );
}

#[tokio::test]
async fn a_link_still_works_shortly_before_it_expires() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let visitor = app.session();
    request_reset(&visitor, "ana@example.com").await;
    let token = token_in(&app.sent_emails()[0]);

    app.advance_time(Duration::minutes(59));

    assert_eq!(
        confirm_reset(&visitor, &token, NEW_PASSWORD).await.status(),
        204
    );
}

#[tokio::test]
async fn an_unknown_token_and_a_short_new_password_are_rejected() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let visitor = app.session();
    request_reset(&visitor, "ana@example.com").await;
    let token = token_in(&app.sent_emails()[0]);

    let unknown = confirm_reset(&visitor, "0123456789abcdef", NEW_PASSWORD).await;
    assert_eq!(unknown.status(), 400);

    let short = confirm_reset(&visitor, &token, "short").await;
    assert_eq!(short.status(), 422);
    let body: Value = short.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["password"],
        json!("password_length")
    );
    assert_eq!(
        confirm_reset(&visitor, &token, NEW_PASSWORD).await.status(),
        204,
        "a refused attempt must not burn the link"
    );
}

#[tokio::test]
async fn asking_again_replaces_the_previous_link() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let visitor = app.session();
    request_reset(&visitor, "ana@example.com").await;
    request_reset(&visitor, "ana@example.com").await;
    let sent = app.sent_emails();
    let (first, second) = (token_in(&sent[0]), token_in(&sent[1]));
    assert_ne!(first, second);

    assert_eq!(
        confirm_reset(&visitor, &first, NEW_PASSWORD).await.status(),
        400
    );
    assert_eq!(
        confirm_reset(&visitor, &second, NEW_PASSWORD)
            .await
            .status(),
        204
    );
}

#[tokio::test]
async fn changing_the_password_in_the_profile_cancels_pending_links() {
    let app = TestApp::spawn().await;
    let ana = app.session();
    ana.sign_up("ana@example.com", PASSWORD).await;
    request_reset(&app.session(), "ana@example.com").await;
    let token = token_in(&app.sent_emails()[0]);

    ana.patch_json(
        "/profile",
        &json!({ "new_password": NEW_PASSWORD, "current_password": PASSWORD }),
    )
    .await;

    let response = confirm_reset(&app.session(), &token, "yet another passphrase").await;
    assert_eq!(response.status(), 400);
}
