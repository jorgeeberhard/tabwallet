//! Changing the e-mail with confirmation (UC14), through the HTTP API with a
//! recording e-mail sender and a clock the test moves by hand: asking for the
//! change only creates a Pending e-mail change; the account keeps its e-mail
//! until the Confirmation link sent to the new address is opened.

mod common;

use chrono::Duration;
use common::{TEST_APP_URL, TestApp};
use reqwest::{Response, header::SET_COOKIE};
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";

async fn error_code(response: Response) -> (u16, String) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    (status, body["error"]["code"].as_str().unwrap().to_string())
}

const LINK: &str = "confirm-email-change?token=";

fn change_links_to(app: &TestApp, to: &str) -> usize {
    let link = format!("{TEST_APP_URL}/{LINK}");
    app.sent_emails()
        .iter()
        .filter(|message| message.to.eq_ignore_ascii_case(to) && message.text.contains(&link))
        .count()
}

/// The "your access e-mail was changed" notices sent to an address.
fn notices_to(app: &TestApp, to: &str) -> usize {
    app.sent_emails()
        .iter()
        .filter(|message| {
            message.to.eq_ignore_ascii_case(to) && message.subject.contains("alterado")
        })
        .count()
}

/// The `/auth/me` answer of a session.
async fn me(browser: &common::Session) -> Value {
    let response = browser.get("/auth/me").await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

// --- Asking for the change --------------------------------------------------

#[tokio::test]
async fn asking_for_the_change_keeps_the_old_e_mail_and_sends_the_link_to_the_new_one() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let sent_before = app.sent_emails().len();

    let response = browser
        .request_email_change(" Ana.New@Example.com ", PASSWORD)
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["user"]["email"], json!("ana@example.com"));
    assert_eq!(
        body["pending_email_change"]["email"],
        json!("Ana.New@Example.com")
    );
    assert!(body["pending_email_change"]["expires_at"].is_string());

    // The old e-mail keeps working; the new one is not an account yet.
    let other = app.session();
    assert_eq!(
        other.log_in("ana@example.com", PASSWORD).await.status(),
        200
    );
    assert_eq!(
        other.log_in("ana.new@example.com", PASSWORD).await.status(),
        401
    );
    // The profile reads the pending change back.
    let current = me(&browser).await;
    assert_eq!(current["user"]["email"], json!("ana@example.com"));
    assert_eq!(
        current["pending_email_change"]["email"],
        json!("Ana.New@Example.com")
    );

    // Only the new address got a message, and it is the link.
    let sent = app.sent_emails().split_off(sent_before);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "Ana.New@Example.com");
    assert!(
        sent[0].text.contains(&format!("{TEST_APP_URL}/{LINK}")),
        "{}",
        sent[0].text
    );
    assert!(sent[0].text.contains("24 horas"), "{}", sent[0].text);
    let token = browser.latest_email_change_token("ana.new@example.com");
    assert_eq!(token.len(), 64);
}

#[tokio::test]
async fn confirming_applies_the_change_notifies_the_old_address_and_starts_no_session() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let token = browser.latest_email_change_token("ana.new@example.com");

    // A visitor (no session) opens the link.
    let visitor = app.session();
    let response = visitor.confirm_email_change(&token).await;

    assert_eq!(response.status(), 204);
    assert!(response.headers().get(SET_COOKIE).is_none());
    assert_eq!(visitor.get("/auth/me").await.status(), 401);

    // The notice goes to the old address, with no link that changes anything.
    let notice = app
        .sent_emails()
        .into_iter()
        .find(|message| message.to == "ana@example.com" && message.subject.contains("alterado"))
        .expect("the old address gets a notice");
    assert!(
        notice.subject.contains("alterado"),
        "subject: {}",
        notice.subject
    );
    assert!(
        notice.text.contains("ana.new@example.com"),
        "{}",
        notice.text
    );
    assert!(!notice.text.contains(LINK), "{}", notice.text);

    // The new e-mail logs in, the old one stops, and the open session lives on.
    let other = app.session();
    assert_eq!(
        other.log_in("ana@example.com", PASSWORD).await.status(),
        401
    );
    assert_eq!(
        other.log_in("ana.new@example.com", PASSWORD).await.status(),
        200
    );
    let current = me(&browser).await;
    assert_eq!(current["user"]["email"], json!("ana.new@example.com"));
    assert!(current["pending_email_change"].is_null());
}

#[tokio::test]
async fn confirming_works_for_a_signed_in_session_too_and_the_session_stays_valid() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let token = browser.latest_email_change_token("ana.new@example.com");

    let response = browser.confirm_email_change(&token).await;

    assert_eq!(response.status(), 204);
    assert!(response.headers().get(SET_COOKIE).is_none());
    assert_eq!(
        me(&browser).await["user"]["email"],
        json!("ana.new@example.com")
    );
}

#[tokio::test]
async fn asking_for_the_change_requires_the_current_password() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let sent_before = app.sent_emails().len();

    let missing = browser
        .patch_json("/profile", &json!({ "email": "ana.new@example.com" }))
        .await;
    assert_eq!(missing.status(), 422);
    let body: Value = missing.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["current_password"],
        json!("required")
    );

    let wrong = browser
        .request_email_change("ana.new@example.com", "not my password")
        .await;
    assert_eq!(
        error_code(wrong).await,
        (403, "invalid_current_password".to_string())
    );

    assert_eq!(app.sent_emails().len(), sent_before);
    assert!(me(&browser).await["pending_email_change"].is_null());
}

#[tokio::test]
async fn the_change_can_only_be_asked_for_while_signed_in() {
    let app = TestApp::spawn().await;
    let visitor = app.session();

    let request = visitor
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let resend = visitor.resend_email_change().await;

    assert_eq!(request.status(), 401);
    assert_eq!(resend.status(), 401);
    assert!(app.sent_emails().is_empty());
}

#[tokio::test]
async fn an_unchanged_e_mail_asks_for_nothing_and_needs_no_password() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let sent_before = app.sent_emails().len();

    let response = browser
        .patch_json(
            "/profile",
            &json!({ "name": "Ana", "email": "Ana@Example.com" }),
        )
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["user"]["name"], json!("Ana"));
    assert_eq!(body["user"]["email"], json!("ana@example.com"));
    assert!(body["pending_email_change"].is_null());
    assert_eq!(app.sent_emails().len(), sent_before);
}

#[tokio::test]
async fn name_and_password_in_the_same_request_apply_at_once_and_only_the_e_mail_waits() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let new_password = "a brand new passphrase";

    let response = browser
        .patch_json(
            "/profile",
            &json!({
                "name": "Ana Souza",
                "email": "ana.new@example.com",
                "new_password": new_password,
                "current_password": PASSWORD
            }),
        )
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["user"]["name"], json!("Ana Souza"));
    assert_eq!(body["user"]["email"], json!("ana@example.com"));
    assert_eq!(
        body["pending_email_change"]["email"],
        json!("ana.new@example.com")
    );
    let other = app.session();
    assert_eq!(
        other.log_in("ana@example.com", PASSWORD).await.status(),
        401
    );
    assert_eq!(
        other.log_in("ana@example.com", new_password).await.status(),
        200
    );
    assert_eq!(change_links_to(&app, "ana.new@example.com"), 1);
}

#[tokio::test]
async fn the_password_policy_still_judges_a_new_password_sent_with_the_e_mail_change() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let sent_before = app.sent_emails().len();

    let response = browser
        .patch_json(
            "/profile",
            &json!({
                "email": "ana.new@example.com",
                "new_password": "1234567890",
                "current_password": PASSWORD
            }),
        )
        .await;

    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["new_password"],
        json!("password_sequence")
    );
    assert_eq!(app.sent_emails().len(), sent_before);
    assert!(me(&browser).await["pending_email_change"].is_null());
}

#[tokio::test]
async fn an_invalid_new_e_mail_is_refused() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let sent_before = app.sent_emails().len();

    let response = browser.request_email_change("nope", PASSWORD).await;

    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["email"], json!("invalid_email"));
    assert_eq!(app.sent_emails().len(), sent_before);
}

// --- The link ---------------------------------------------------------------

#[tokio::test]
async fn the_link_expires_after_24_hours_and_the_old_e_mail_stays() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let token = browser.latest_email_change_token("ana.new@example.com");

    app.advance_time(Duration::hours(24) + Duration::minutes(1));
    let response = browser.confirm_email_change(&token).await;

    assert_eq!(
        error_code(response).await,
        (400, "invalid_confirmation_token".to_string())
    );
    let current = me(&browser).await;
    assert_eq!(current["user"]["email"], json!("ana@example.com"));
    // An expired change is not shown as pending, and cannot be resent.
    assert!(current["pending_email_change"].is_null());
    assert_eq!(browser.resend_email_change().await.status(), 404);
}

#[tokio::test]
async fn the_link_works_once() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let token = browser.latest_email_change_token("ana.new@example.com");
    assert_eq!(browser.confirm_email_change(&token).await.status(), 204);

    let again = browser.confirm_email_change(&token).await;

    assert_eq!(
        error_code(again).await,
        (400, "invalid_confirmation_token".to_string())
    );
    // Only one notice went to the old address.
    assert_eq!(notices_to(&app, "ana@example.com"), 1);
}

#[tokio::test]
async fn an_unknown_link_is_refused_the_same_way_and_a_blank_one_is_invalid() {
    let app = TestApp::spawn().await;
    let browser = app.session();

    let unknown = browser.confirm_email_change(&"0".repeat(64)).await;
    let blank = browser.confirm_email_change("  ").await;

    assert_eq!(
        error_code(unknown).await,
        (400, "invalid_confirmation_token".to_string())
    );
    assert_eq!(blank.status(), 422);
}

#[tokio::test]
async fn a_new_request_replaces_the_previous_one_and_kills_its_link() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("first@example.com", PASSWORD)
        .await;
    let first = browser.latest_email_change_token("first@example.com");
    browser
        .request_email_change("second@example.com", PASSWORD)
        .await;
    let second = browser.latest_email_change_token("second@example.com");

    let stale = browser.confirm_email_change(&first).await;
    assert_eq!(
        error_code(stale).await,
        (400, "invalid_confirmation_token".to_string())
    );
    assert_eq!(browser.confirm_email_change(&second).await.status(), 204);

    let current = me(&browser).await;
    assert_eq!(current["user"]["email"], json!("second@example.com"));
    // There was only ever one pending change: the first address stays free.
    assert_eq!(
        app.session()
            .log_in("first@example.com", PASSWORD)
            .await
            .status(),
        401
    );
}

#[tokio::test]
async fn asking_again_for_the_same_address_also_replaces_the_link() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let first = browser.latest_email_change_token("ana.new@example.com");
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let second = browser.latest_email_change_token("ana.new@example.com");

    assert_ne!(first, second);
    assert_eq!(browser.confirm_email_change(&first).await.status(), 400);
    assert_eq!(browser.confirm_email_change(&second).await.status(), 204);
}

// --- Resending and the limit ------------------------------------------------

#[tokio::test]
async fn resending_sends_a_new_link_to_the_pending_address_and_the_old_link_stops() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let first = browser.latest_email_change_token("ana.new@example.com");

    let response = browser.resend_email_change().await;

    assert_eq!(response.status(), 202);
    assert_eq!(change_links_to(&app, "ana.new@example.com"), 2);
    assert_eq!(notices_to(&app, "ana@example.com"), 0);
    let second = browser.latest_email_change_token("ana.new@example.com");
    assert_ne!(first, second);
    assert_eq!(browser.confirm_email_change(&first).await.status(), 400);
    assert_eq!(browser.confirm_email_change(&second).await.status(), 204);
}

#[tokio::test]
async fn resending_without_a_pending_change_is_not_found() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let sent_before = app.sent_emails().len();

    let response = browser.resend_email_change().await;

    assert_eq!(
        error_code(response).await,
        (404, "no_pending_email_change".to_string())
    );
    assert_eq!(app.sent_emails().len(), sent_before);
}

#[tokio::test]
async fn requests_and_resends_share_a_limit_of_three_an_hour_for_the_account() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    assert_eq!(
        browser
            .request_email_change("ana.new@example.com", PASSWORD)
            .await
            .status(),
        200
    );
    assert_eq!(browser.resend_email_change().await.status(), 202);
    assert_eq!(
        browser
            .request_email_change("ana.other@example.com", PASSWORD)
            .await
            .status(),
        200
    );
    let sent_before = app.sent_emails().len();

    let resend = browser.resend_email_change().await;
    let request = browser
        .request_email_change("ana.third@example.com", PASSWORD)
        .await;

    for response in [resend, request] {
        assert_eq!(response.status(), 429);
        assert!(response.headers().contains_key("retry-after"));
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], json!("too_many_requests"));
    }
    assert_eq!(app.sent_emails().len(), sent_before);
    // A refused request changes nothing: the second address is still pending.
    assert_eq!(
        me(&browser).await["pending_email_change"]["email"],
        json!("ana.other@example.com")
    );

    app.advance_time(Duration::hours(1) + Duration::minutes(1));
    assert_eq!(browser.resend_email_change().await.status(), 202);
}

#[tokio::test]
async fn the_limit_is_per_account_and_refused_changes_do_not_count() {
    let app = TestApp::spawn().await;
    let ana = app.session();
    ana.sign_up("ana@example.com", PASSWORD).await;
    let bia = app.session();
    bia.sign_up("bia@example.com", PASSWORD).await;
    for _ in 0..5 {
        ana.request_email_change("ana.new@example.com", "not my password")
            .await;
    }
    for _ in 0..3 {
        ana.request_email_change("ana.new@example.com", PASSWORD)
            .await;
    }

    assert_eq!(
        ana.request_email_change("ana.new@example.com", PASSWORD)
            .await
            .status(),
        429
    );
    assert_eq!(
        bia.request_email_change("bia.new@example.com", PASSWORD)
            .await
            .status(),
        200
    );
}

// --- Conflicts and anti-enumeration -----------------------------------------

#[tokio::test]
async fn confirming_is_refused_when_the_new_e_mail_became_an_account_meanwhile() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    let token = browser.latest_email_change_token("ana.new@example.com");
    // Someone else signs up and confirms the very address.
    app.session().sign_up("ana.new@example.com", PASSWORD).await;
    let notices_before = notices_to(&app, "ana@example.com");

    let response = browser.confirm_email_change(&token).await;

    assert_eq!(error_code(response).await, (409, "email_taken".to_string()));
    assert_eq!(
        me(&browser).await["user"]["email"],
        json!("ana@example.com")
    );
    assert_eq!(
        app.session()
            .log_in("ana@example.com", PASSWORD)
            .await
            .status(),
        200
    );
    // The old address is told nothing, because nothing changed.
    assert_eq!(notices_to(&app, "ana@example.com"), notices_before);
}

#[tokio::test]
async fn asking_for_an_address_that_has_an_account_answers_like_a_pending_change_and_sends_a_notice()
 {
    let app = TestApp::spawn().await;
    app.session().sign_up("bia@example.com", PASSWORD).await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    let sent_before = app.sent_emails().len();

    let taken = browser
        .request_email_change("BIA@example.com", PASSWORD)
        .await;
    let fresh = browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;

    // Same status and same shape: the answer does not tell the two apart.
    assert_eq!(taken.status(), 200);
    assert_eq!(fresh.status(), 200);
    let taken: Value = taken.json().await.unwrap();
    let fresh: Value = fresh.json().await.unwrap();
    assert_eq!(taken["user"], fresh["user"]);
    assert_eq!(
        taken["pending_email_change"]["email"],
        json!("BIA@example.com")
    );
    assert_eq!(
        taken["pending_email_change"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        fresh["pending_email_change"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>()
    );

    // Bia got the "you already have an account" notice and no link.
    let to_bia: Vec<_> = app.sent_emails()[sent_before..]
        .iter()
        .filter(|message| message.to.eq_ignore_ascii_case("bia@example.com"))
        .cloned()
        .collect();
    assert_eq!(to_bia.len(), 1);
    assert!(to_bia[0].subject.contains("já tem uma conta"));
    assert!(!to_bia[0].text.contains(LINK), "{}", to_bia[0].text);
    assert_eq!(change_links_to(&app, "bia@example.com"), 0);
}

#[tokio::test]
async fn the_pending_change_to_an_address_with_an_account_never_applies() {
    let app = TestApp::spawn().await;
    app.session().sign_up("bia@example.com", PASSWORD).await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    browser
        .request_email_change("bia@example.com", PASSWORD)
        .await;

    // Looks pending in the profile, like any other.
    assert_eq!(
        me(&browser).await["pending_email_change"]["email"],
        json!("bia@example.com")
    );
    // Resending answers the same and sends Bia the notice again, never a link.
    assert_eq!(browser.resend_email_change().await.status(), 202);
    assert_eq!(change_links_to(&app, "bia@example.com"), 0);
    assert_eq!(
        app.sent_emails()
            .iter()
            .filter(|message| message.to == "bia@example.com"
                && message.subject.contains("já tem uma conta"))
            .count(),
        2
    );
    // And the old e-mail is still Ana's, with Bia's account untouched.
    assert_eq!(
        me(&browser).await["user"]["email"],
        json!("ana@example.com")
    );
    assert_eq!(
        app.session()
            .log_in("bia@example.com", PASSWORD)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn resending_after_the_address_became_an_account_sends_the_notice_not_a_link() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;
    browser
        .request_email_change("ana.new@example.com", PASSWORD)
        .await;
    app.session().sign_up("ana.new@example.com", PASSWORD).await;

    let response = browser.resend_email_change().await;

    assert_eq!(response.status(), 202);
    assert_eq!(change_links_to(&app, "ana.new@example.com"), 1);
    let last = app.sent_emails().pop().unwrap();
    assert_eq!(last.to, "ana.new@example.com");
    assert!(
        last.subject.contains("já tem uma conta"),
        "{}",
        last.subject
    );
}
