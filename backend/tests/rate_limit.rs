//! Attempt limits on login (failed attempts) and on password-reset requests.

mod common;

use chrono::Duration;
use common::TestApp;
use reqwest::header::RETRY_AFTER;
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";

async fn code(response: reqwest::Response) -> String {
    let body: Value = response.json().await.unwrap();
    body["error"]["code"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn five_wrong_passwords_block_the_email_even_for_the_right_password() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let browser = app.session();

    for _ in 0..5 {
        let response = browser.log_in("ana@example.com", "wrong password").await;
        assert_eq!(response.status(), 401);
    }
    let blocked = browser.log_in("ana@example.com", PASSWORD).await;

    assert_eq!(blocked.status(), 429);
    let seconds: u64 = blocked.headers()[RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=15 * 60).contains(&seconds), "Retry-After {seconds}");
    assert_eq!(code(blocked).await, "too_many_requests");
    // The block does not depend on the letter case of the address.
    let blocked_again = browser.log_in("ANA@example.com", PASSWORD).await;
    assert_eq!(blocked_again.status(), 429);
}

#[tokio::test]
async fn the_block_ends_after_fifteen_minutes() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let browser = app.session();
    for _ in 0..5 {
        browser.log_in("ana@example.com", "wrong password").await;
    }
    assert_eq!(
        browser.log_in("ana@example.com", PASSWORD).await.status(),
        429
    );

    app.advance_time(Duration::minutes(16));

    assert_eq!(
        browser.log_in("ana@example.com", PASSWORD).await.status(),
        200
    );
}

#[tokio::test]
async fn a_successful_login_resets_the_count() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let browser = app.session();

    for _ in 0..4 {
        browser.log_in("ana@example.com", "wrong password").await;
    }
    assert_eq!(
        browser.log_in("ana@example.com", PASSWORD).await.status(),
        200
    );
    for _ in 0..4 {
        let response = browser.log_in("ana@example.com", "wrong password").await;
        assert_eq!(response.status(), 401);
    }
}

#[tokio::test]
async fn one_email_being_blocked_does_not_block_the_others() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    app.session().sign_up("bia@example.com", PASSWORD).await;
    let browser = app.session();
    for _ in 0..5 {
        browser.log_in("ana@example.com", "wrong password").await;
    }

    assert_eq!(
        browser.log_in("bia@example.com", PASSWORD).await.status(),
        200
    );
}

#[tokio::test]
async fn an_unknown_email_is_limited_the_same_way() {
    let app = TestApp::spawn().await;
    let browser = app.session();

    for _ in 0..5 {
        let response = browser.log_in("ghost@example.com", "whatever 123").await;
        assert_eq!(response.status(), 401);
    }

    let blocked = browser.log_in("ghost@example.com", "whatever 123").await;
    assert_eq!(blocked.status(), 429);
}

#[tokio::test]
async fn the_fourth_password_reset_request_in_an_hour_is_refused_and_sends_nothing() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let browser = app.session();
    let request = async |email: &str| {
        browser
            .post_json("/auth/password-reset/request", &json!({ "email": email }))
            .await
    };

    for _ in 0..3 {
        assert_eq!(request("ana@example.com").await.status(), 202);
    }
    let refused = request("ana@example.com").await;

    assert_eq!(refused.status(), 429);
    assert!(refused.headers().contains_key(RETRY_AFTER));
    assert_eq!(app.sent_emails().len(), 3);
    // An address without an account is counted the same way.
    for _ in 0..3 {
        assert_eq!(request("ghost@example.com").await.status(), 202);
    }
    assert_eq!(request("ghost@example.com").await.status(), 429);

    app.advance_time(Duration::minutes(61));
    assert_eq!(request("ana@example.com").await.status(), 202);
}
