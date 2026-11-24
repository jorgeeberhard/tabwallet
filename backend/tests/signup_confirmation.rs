//! Sign-up with e-mail confirmation (UC01), through the HTTP API with a
//! recording e-mail sender and a clock the test moves by hand: asking for a
//! sign-up only creates a Pending signup; the Account exists once the
//! Confirmation link is opened.

mod common;

use chrono::Duration;
use common::{Session, TEST_APP_URL, TestApp};
use reqwest::{Response, header::SET_COOKIE};
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";

async fn error_code(response: Response) -> (u16, String) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    (status, body["error"]["code"].as_str().unwrap().to_string())
}

fn confirmation_emails_to(app: &TestApp, to: &str) -> usize {
    let link = format!("{TEST_APP_URL}/confirm-email?token=");
    app.sent_emails()
        .iter()
        .filter(|message| message.to.eq_ignore_ascii_case(to) && message.text.contains(&link))
        .count()
}

/// Counts the Pending signups. The only test that looks past the HTTP API: the
/// deletion of expired ones is not otherwise observable.
async fn pending_count(app: &TestApp) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM pending_signups")
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

// --- Asking for a sign-up ---------------------------------------------------

#[tokio::test]
async fn asking_for_a_sign_up_sends_the_confirmation_link_and_signs_nobody_in() {
    let app = TestApp::spawn().await;
    let browser = app.session();

    let response = browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;

    assert_eq!(response.status(), 202);
    assert!(response.headers().get(SET_COOKIE).is_none());
    assert_eq!(browser.get("/auth/me").await.status(), 401);
    let sent = app.sent_emails();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "ana@example.com");
    assert!(
        sent[0]
            .text
            .contains(&format!("{TEST_APP_URL}/confirm-email?token=")),
        "{}",
        sent[0].text
    );
    assert!(sent[0].text.contains("24 horas"), "{}", sent[0].text);
    let token = browser.latest_confirmation_token("ana@example.com");
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn a_pending_sign_up_is_not_an_account_so_it_cannot_use_the_system() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;

    let response = browser.log_in("ana@example.com", PASSWORD).await;

    assert!(response.headers().get(SET_COOKIE).is_none());
    assert_eq!(browser.get("/auth/me").await.status(), 401);
}

#[tokio::test]
async fn asking_for_a_sign_up_validates_name_and_e_mail() {
    let app = TestApp::spawn().await;
    let browser = app.session();

    let blank_name = browser
        .request_sign_up("ana@example.com", "   ", PASSWORD)
        .await;
    assert_eq!(blank_name.status(), 422);
    let body: Value = blank_name.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["name"], json!("required"));

    let long_name = browser
        .request_sign_up("ana@example.com", &"a".repeat(101), PASSWORD)
        .await;
    assert_eq!(long_name.status(), 422);
    let body: Value = long_name.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["name"], json!("name_length"));

    let bad_email = browser
        .request_sign_up("not-an-email", "Ana Souza", PASSWORD)
        .await;
    assert_eq!(bad_email.status(), 422);
    let body: Value = bad_email.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["email"], json!("invalid_email"));
    assert!(app.sent_emails().is_empty());
}

#[tokio::test]
async fn the_password_policy_judges_the_password_against_the_name_and_e_mail_of_the_sign_up() {
    let app = TestApp::spawn().await;
    let browser = app.session();

    let cases = [
        (
            "Beatriz Souza",
            "xk!beatriz#92pass",
            "password_contains_name",
        ),
        ("Ana Souza", "souza-is-my-family", "password_contains_name"),
        ("Ana Souza", "xk!bruno.lima#92", "password_contains_email"),
        ("Ana Souza", "12345678", "password_too_short"),
        ("Ana Souza", "password1234", "password_common"),
    ];
    for (name, password, code) in cases {
        let response = browser
            .request_sign_up("bruno.lima@example.com", name, password)
            .await;
        assert_eq!(response.status(), 422, "{password:?} should be refused");
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["fields"]["password"], json!(code));
    }
    assert!(app.sent_emails().is_empty());
    assert_eq!(pending_count(&app).await, 0);
}

// --- Confirming -------------------------------------------------------------

#[tokio::test]
async fn confirming_creates_the_account_with_its_default_categories_but_does_not_sign_in() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;
    let token = browser.latest_confirmation_token("ana@example.com");

    let confirmed = browser.confirm_sign_up(&token).await;

    assert_eq!(confirmed.status(), 204);
    assert!(confirmed.headers().get(SET_COOKIE).is_none());
    assert_eq!(browser.get("/auth/me").await.status(), 401);
    let login = browser.log_in("ana@example.com", PASSWORD).await;
    assert_eq!(login.status(), 200);
    let body: Value = login.json().await.unwrap();
    assert_eq!(body["user"]["email"], json!("ana@example.com"));
    assert_eq!(body["user"]["name"], json!("Ana Souza"));
    let categories: Value = browser.get("/categories").await.json().await.unwrap();
    let names: Vec<&str> = categories["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|category| category["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Salário"), "{names:?}");
    assert!(names.contains(&"Juros e encargos"), "{names:?}");
    assert_eq!(pending_count(&app).await, 0);
}

#[tokio::test]
async fn the_link_works_once() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;
    let token = browser.latest_confirmation_token("ana@example.com");
    assert_eq!(browser.confirm_sign_up(&token).await.status(), 204);

    let again = browser.confirm_sign_up(&token).await;

    assert_eq!(
        error_code(again).await,
        (400, "invalid_confirmation_token".to_string())
    );
}

#[tokio::test]
async fn an_unknown_token_is_refused_like_a_used_one() {
    let app = TestApp::spawn().await;

    let response = app.session().confirm_sign_up(&"0".repeat(64)).await;

    assert_eq!(
        error_code(response).await,
        (400, "invalid_confirmation_token".to_string())
    );
}

#[tokio::test]
async fn the_link_lasts_24_hours() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;
    browser
        .request_sign_up("bia@example.com", "Bia Lima", PASSWORD)
        .await;
    let ana_token = browser.latest_confirmation_token("ana@example.com");
    let bia_token = browser.latest_confirmation_token("bia@example.com");

    app.advance_time(Duration::hours(23) + Duration::minutes(59));
    assert_eq!(browser.confirm_sign_up(&ana_token).await.status(), 204);
    app.advance_time(Duration::minutes(2));
    let late = browser.confirm_sign_up(&bia_token).await;

    assert_eq!(
        error_code(late).await,
        (400, "invalid_confirmation_token".to_string())
    );
    assert_eq!(
        browser.log_in("bia@example.com", PASSWORD).await.status(),
        401,
        "an expired Pending signup is as good as none"
    );
}

#[tokio::test]
async fn confirming_with_an_e_mail_that_became_an_account_meanwhile_is_refused() {
    let app = TestApp::spawn().await;
    let ana = app.session();
    ana.request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;
    let token = ana.latest_confirmation_token("ana@example.com");
    // Someone else's account takes the address in the meantime (the profile
    // still changes the e-mail directly).
    let bruno = app.session();
    bruno.sign_up("bruno@example.com", PASSWORD).await;
    let taken = bruno
        .patch_json(
            "/profile",
            &json!({ "email": "Ana@Example.com", "current_password": PASSWORD }),
        )
        .await;
    assert_eq!(taken.status(), 200);

    let response = ana.confirm_sign_up(&token).await;

    assert_eq!(error_code(response).await, (409, "email_taken".to_string()));
}

// --- Replacing a Pending signup ---------------------------------------------

#[tokio::test]
async fn a_new_sign_up_with_the_same_e_mail_replaces_the_pending_one_and_kills_the_old_link() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", "first password here")
        .await;
    let old_token = browser.latest_confirmation_token("ana@example.com");

    let second = browser
        .request_sign_up("ANA@example.com", "Ana S. Lima", "second password here")
        .await;
    let new_token = browser.latest_confirmation_token("ana@example.com");

    assert_eq!(second.status(), 202);
    assert_ne!(old_token, new_token);
    assert_eq!(pending_count(&app).await, 1);
    assert_eq!(
        error_code(browser.confirm_sign_up(&old_token).await).await,
        (400, "invalid_confirmation_token".to_string())
    );
    assert_eq!(browser.confirm_sign_up(&new_token).await.status(), 204);
    assert_eq!(
        browser
            .log_in("ana@example.com", "first password here")
            .await
            .status(),
        401
    );
    let login = browser
        .log_in("ana@example.com", "second password here")
        .await;
    assert_eq!(login.status(), 200);
    let body: Value = login.json().await.unwrap();
    assert_eq!(body["user"]["name"], json!("Ana S. Lima"));
}

#[tokio::test]
async fn a_pending_sign_up_does_not_stop_the_real_owner_from_signing_up() {
    let app = TestApp::spawn().await;
    // A stranger asks for a sign-up with Ana's address and a password of theirs.
    app.session()
        .request_sign_up("ana@example.com", "Impostor", "the impostor password")
        .await;
    let ana = app.session();

    let response = ana
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;
    let token = ana.latest_confirmation_token("ana@example.com");

    assert_eq!(response.status(), 202);
    assert_eq!(ana.confirm_sign_up(&token).await.status(), 204);
    assert_eq!(ana.log_in("ana@example.com", PASSWORD).await.status(), 200);
    assert_eq!(
        app.session()
            .log_in("ana@example.com", "the impostor password")
            .await
            .status(),
        401
    );
}

#[tokio::test]
async fn expired_pending_sign_ups_are_ignored_and_deleted_when_a_new_sign_up_arrives() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("old@example.com", "Old Timer", PASSWORD)
        .await;
    app.advance_time(Duration::hours(25));
    assert_eq!(pending_count(&app).await, 1, "nothing deletes it by itself");

    browser
        .request_sign_up("new@example.com", "Newcomer Lee", PASSWORD)
        .await;

    assert_eq!(pending_count(&app).await, 1, "only the new one is left");
}

// --- An e-mail that already has an account ----------------------------------

#[tokio::test]
async fn a_sign_up_with_the_e_mail_of_an_account_looks_the_same_and_sends_the_notice() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let before = app.sent_emails().len();
    let visitor = app.session();

    let existing = visitor
        .request_sign_up("ANA@example.com", "Someone Else", "some other password")
        .await;
    let existing_status = existing.status();
    let existing_cookie = existing.headers().get(SET_COOKIE).cloned();
    let existing_body = existing.text().await.unwrap();
    let new = app
        .session()
        .request_sign_up("brand.new@example.com", "Brand New", PASSWORD)
        .await;

    assert_eq!(existing_status, 202);
    assert!(existing_cookie.is_none());
    assert_eq!(
        (new.status().as_u16(), new.text().await.unwrap()),
        (202, existing_body),
        "the answer must not reveal that the account exists"
    );
    let sent = app.sent_emails();
    let notice = &sent[before];
    assert_eq!(notice.to, "ana@example.com");
    assert!(
        !notice.text.contains("/confirm-email"),
        "the notice carries no confirmation link: {}",
        notice.text
    );
    assert!(notice.text.contains("já tem uma conta"), "{}", notice.text);
    assert!(
        notice.text.contains(&format!("{TEST_APP_URL}/login"))
            && notice
                .text
                .contains(&format!("{TEST_APP_URL}/forgot-password")),
        "{}",
        notice.text
    );
    // The account is untouched: still the old password, and no pending
    // sign-up was created for its address.
    assert_eq!(
        app.session()
            .log_in("ana@example.com", "some other password")
            .await
            .status(),
        401
    );
    assert_eq!(
        app.session()
            .log_in("ana@example.com", PASSWORD)
            .await
            .status(),
        200
    );
    assert_eq!(pending_count(&app).await, 1, "brand.new only");
}

// --- Resending --------------------------------------------------------------

#[tokio::test]
async fn resending_sends_a_new_link_and_the_old_one_stops_working() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;
    let old_token = browser.latest_confirmation_token("ana@example.com");

    let response = browser.resend_confirmation("ANA@example.com").await;
    let new_token = browser.latest_confirmation_token("ana@example.com");

    assert_eq!(response.status(), 202);
    assert_eq!(confirmation_emails_to(&app, "ana@example.com"), 2);
    assert_ne!(old_token, new_token);
    assert_eq!(
        error_code(browser.confirm_sign_up(&old_token).await).await,
        (400, "invalid_confirmation_token".to_string())
    );
    assert_eq!(browser.confirm_sign_up(&new_token).await.status(), 204);
    // The password chosen at sign-up is kept.
    assert_eq!(
        browser.log_in("ana@example.com", PASSWORD).await.status(),
        200
    );
}

#[tokio::test]
async fn resending_answers_the_same_for_any_address_and_sends_only_for_a_pending_one() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let browser = app.session();
    browser
        .request_sign_up("bia@example.com", "Bia Lima", PASSWORD)
        .await;
    let before = app.sent_emails().len();

    let pending = browser.resend_confirmation("bia@example.com").await;
    let pending = (pending.status(), pending.text().await.unwrap());
    let account = browser.resend_confirmation("ana@example.com").await;
    let account = (account.status(), account.text().await.unwrap());
    let nobody = browser.resend_confirmation("nobody@example.com").await;
    let nobody = (nobody.status(), nobody.text().await.unwrap());

    assert_eq!(pending.0, 202);
    assert_eq!(pending, account);
    assert_eq!(pending, nobody);
    assert_eq!(app.sent_emails().len(), before + 1, "only Bia's is sent");
}

#[tokio::test]
async fn an_expired_pending_sign_up_is_not_resent() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;
    app.advance_time(Duration::hours(25));

    let response = browser.resend_confirmation("ana@example.com").await;

    assert_eq!(response.status(), 202);
    assert_eq!(app.sent_emails().len(), 1);
}

#[tokio::test]
async fn the_fourth_resend_in_an_hour_is_refused_and_sends_nothing() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;

    for _ in 0..3 {
        assert_eq!(
            browser
                .resend_confirmation("ana@example.com")
                .await
                .status(),
            202
        );
    }
    let refused = browser.resend_confirmation("ana@example.com").await;
    let retry_after = refused.headers().get("retry-after").cloned();
    let sent_before = app.sent_emails().len();

    assert_eq!(refused.status(), 429);
    assert!(retry_after.is_some());
    assert_eq!(sent_before, 4, "sign-up plus three resends");
    // The limit counts every address, so it reveals nothing either.
    for _ in 0..3 {
        browser.resend_confirmation("nobody@example.com").await;
    }
    assert_eq!(
        browser
            .resend_confirmation("NOBODY@example.com")
            .await
            .status(),
        429
    );
    // Another address is not affected, and the hour ends the block.
    assert_eq!(
        browser
            .resend_confirmation("bia@example.com")
            .await
            .status(),
        202
    );
    app.advance_time(Duration::minutes(61));
    assert_eq!(
        browser
            .resend_confirmation("ana@example.com")
            .await
            .status(),
        202
    );
    assert_eq!(app.sent_emails().len(), sent_before + 1);
}

#[tokio::test]
async fn the_fourth_sign_up_request_for_an_address_in_an_hour_is_refused_and_sends_nothing() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    for _ in 0..3 {
        assert_eq!(
            browser
                .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
                .await
                .status(),
            202
        );
    }

    let refused = browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;

    assert_eq!(refused.status(), 429);
    assert_eq!(app.sent_emails().len(), 3);
    // A refusal by the password rule does not use up an attempt.
    for _ in 0..3 {
        let weak = browser
            .request_sign_up("bia@example.com", "Bia Lima", "short")
            .await;
        assert_eq!(weak.status(), 422);
    }
    for _ in 0..3 {
        assert_eq!(
            browser
                .request_sign_up("bia@example.com", "Bia Lima", PASSWORD)
                .await
                .status(),
            202
        );
    }
}

// --- Login ------------------------------------------------------------------

async fn pending_login_session(app: &TestApp) -> Session {
    let browser = app.session();
    browser
        .request_sign_up("ana@example.com", "Ana Souza", PASSWORD)
        .await;
    browser
}

#[tokio::test]
async fn login_tells_a_pending_sign_up_to_confirm_the_e_mail_only_when_e_mail_and_password_match() {
    let app = TestApp::spawn().await;
    let browser = pending_login_session(&app).await;

    let matching = browser.log_in("Ana@Example.com", PASSWORD).await;

    assert!(matching.headers().get(SET_COOKIE).is_none());
    assert_eq!(
        error_code(matching).await,
        (403, "email_not_confirmed".to_string())
    );
}

#[tokio::test]
async fn login_stays_generic_for_a_pending_sign_up_with_the_wrong_password_and_for_strangers() {
    let app = TestApp::spawn().await;
    app.session().sign_up("bia@example.com", PASSWORD).await;
    let browser = pending_login_session(&app).await;

    let wrong_password = browser.log_in("ana@example.com", "not the password").await;
    let unknown = browser.log_in("nobody@example.com", PASSWORD).await;
    let account_wrong_password = browser.log_in("bia@example.com", "not the password").await;

    for response in [wrong_password, unknown, account_wrong_password] {
        assert_eq!(
            error_code(response).await,
            (401, "invalid_credentials".to_string())
        );
    }
}

#[tokio::test]
async fn login_of_a_confirmed_account_is_unchanged_even_if_someone_signed_up_with_its_e_mail() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    // Someone asks for a sign-up with Ana's address: the notice goes out, no
    // pending one is created, so the account logs in as always.
    app.session()
        .request_sign_up("ana@example.com", "Someone Else", "some other password")
        .await;
    let browser = app.session();

    let response = browser.log_in("ana@example.com", PASSWORD).await;
    let other = browser
        .log_in("ana@example.com", "some other password")
        .await;

    assert_eq!(response.status(), 200);
    assert!(response.headers().get(SET_COOKIE).is_some());
    assert_eq!(
        error_code(other).await,
        (401, "invalid_credentials".to_string())
    );
}
