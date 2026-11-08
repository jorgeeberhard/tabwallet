//! Cross-site request forgery: the session cookie is `SameSite=Lax`, and on top
//! of that the API refuses state-changing requests whose `Origin` (or, without
//! one, `Referer`) is not the front-end. Safe requests (GET) are not checked.

mod common;

use common::{Session, TEST_APP_URL, TestApp};
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";

async fn error_code(response: reqwest::Response) -> (u16, String) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    (status, body["error"]["code"].as_str().unwrap().to_string())
}

async fn signed_up(app: &TestApp) -> Session {
    let browser = app.session();
    assert_eq!(
        browser.sign_up("ana@example.com", PASSWORD).await.status(),
        201
    );
    browser
}

fn new_card() -> Value {
    json!({ "name": "Nubank", "closing_day": 10, "due_day": 17 })
}

#[tokio::test]
async fn a_state_changing_request_from_another_site_is_refused_and_changes_nothing() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app).await;

    let response = ana
        .post_json_with_header(
            "/credit-cards",
            &new_card(),
            "Origin",
            "https://evil.example",
        )
        .await;

    assert_eq!(error_code(response).await, (403, "forbidden_origin".into()));
    let cards: Value = ana.get("/credit-cards").await.json().await.unwrap();
    assert_eq!(cards["credit_cards"], json!([]));
}

#[tokio::test]
async fn the_front_end_origin_is_accepted() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app).await;

    let response = ana
        .post_json_with_header("/credit-cards", &new_card(), "Origin", TEST_APP_URL)
        .await;

    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn a_request_without_origin_or_referer_is_accepted() {
    // curl, tests and old browsers do not send them; SameSite=Lax still
    // keeps the cookie off cross-site requests.
    let app = TestApp::spawn().await;
    let ana = signed_up(&app).await;

    let response = ana.post_json("/credit-cards", &new_card()).await;

    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn the_null_origin_of_a_sandboxed_page_is_refused() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app).await;

    let response = ana
        .post_json_with_header("/credit-cards", &new_card(), "Origin", "null")
        .await;

    assert_eq!(error_code(response).await, (403, "forbidden_origin".into()));
}

#[tokio::test]
async fn a_look_alike_origin_is_refused() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app).await;

    for origin in [
        "http://app.test.evil.example",
        "https://app.test",
        "http://app.test:8080",
    ] {
        let response = ana
            .post_json_with_header("/credit-cards", &new_card(), "Origin", origin)
            .await;
        assert_eq!(
            error_code(response).await,
            (403, "forbidden_origin".into()),
            "{origin}"
        );
    }
}

#[tokio::test]
async fn without_origin_the_referer_decides() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app).await;

    let foreign = ana
        .post_json_with_header(
            "/credit-cards",
            &new_card(),
            "Referer",
            "https://evil.example/page",
        )
        .await;
    assert_eq!(error_code(foreign).await, (403, "forbidden_origin".into()));

    let own = ana
        .post_json_with_header(
            "/credit-cards",
            &new_card(),
            "Referer",
            &format!("{TEST_APP_URL}/cartoes"),
        )
        .await;
    assert_eq!(own.status(), 201);
}

#[tokio::test]
async fn login_and_logout_are_checked_too_but_reads_are_not() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app).await;

    let login = ana
        .post_json_with_header(
            "/auth/login",
            &json!({ "email": "ana@example.com", "password": PASSWORD }),
            "Origin",
            "https://evil.example",
        )
        .await;
    assert_eq!(error_code(login).await, (403, "forbidden_origin".into()));

    let logout = ana
        .post_json_with_header("/auth/logout", &json!({}), "Origin", "https://evil.example")
        .await;
    assert_eq!(error_code(logout).await, (403, "forbidden_origin".into()));

    // A GET never changes anything, so a foreign Origin does not matter.
    let me = ana
        .get_with_header("/auth/me", "Origin", "https://evil.example")
        .await;
    assert_eq!(me.status(), 200);
}
