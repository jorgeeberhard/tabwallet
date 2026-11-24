//! Sign-up, login, logout and protected routes (UC01, UC02), through the HTTP API.

mod common;

use common::{SESSION_COOKIE, TEST_JWT_SECRET, TestApp};
use reqwest::header::SET_COOKIE;
use serde_json::{Value, json};

const PASSWORD: &str = "Correct horse battery 7";

/// The sign-up itself (Pending signup, confirmation, replacement, notice,
/// resending) is covered in `signup_confirmation.rs`; here only the rules that
/// do not depend on it.
#[tokio::test]
async fn login_starts_a_session_with_an_http_only_cookie() {
    let app = TestApp::spawn().await;
    let browser = app.session();

    let response = browser.sign_up("ana@example.com", PASSWORD).await;

    assert_eq!(response.status(), 200);
    let set_cookie = response
        .headers()
        .get(SET_COOKIE)
        .expect("login must set the session cookie")
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.starts_with(&format!("{SESSION_COOKIE}=")));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Path=/"));
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["user"]["email"], json!("ana@example.com"));
    assert!(body["user"]["id"].is_string());
    assert!(
        !body.to_string().to_lowercase().contains("password"),
        "the response must not echo password data: {body}"
    );
}

/// Signs up with a password the Password policy must refuse and checks the
/// stable code under the `password` field, and that nobody is signed in.
async fn assert_signup_refuses(email: &str, password: &str, code: &str) {
    let app = TestApp::spawn().await;
    let browser = app.session();

    let response = browser.request_sign_up(email, "Ana Souza", password).await;

    assert_eq!(response.status(), 422, "{password:?} should be refused");
    assert!(response.headers().get(SET_COOKIE).is_none());
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("validation_failed"));
    assert_eq!(
        body["error"]["fields"]["password"],
        json!(code),
        "wrong code for {password:?}"
    );
    assert_eq!(browser.get("/auth/me").await.status(), 401);
}

#[tokio::test]
async fn signup_refuses_a_password_shorter_than_ten_characters() {
    for weak in ["short", "12345678", "password", "senha123", "xk!bruno9"] {
        assert_signup_refuses("ana@example.com", weak, "password_too_short").await;
    }
}

#[tokio::test]
async fn signup_refuses_a_password_longer_than_128_characters() {
    let too_long = "the quick brown fox jumps over the lazy dog ".repeat(3);
    assert!(too_long.chars().count() > 128);
    assert_signup_refuses("ana@example.com", &too_long, "password_too_long").await;
}

#[tokio::test]
async fn signup_counts_characters_not_bytes_for_the_length() {
    let app = TestApp::spawn().await;

    // Nine characters, but more than ten bytes.
    let nine = app
        .session()
        .request_sign_up("ana@example.com", "Ana Souza", "çãõéíóúàê")
        .await;
    assert_eq!(nine.status(), 422);
    let body: Value = nine.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["password"],
        json!("password_too_short")
    );

    // Exactly ten characters.
    let ten = app
        .session()
        .request_sign_up("ana@example.com", "Ana Souza", "Çãõéíóúàê1")
        .await;
    assert_eq!(ten.status(), 202);
}

#[tokio::test]
async fn signup_refuses_common_passwords_ignoring_case_and_trivial_endings() {
    for common in [
        "1Q2w3e4r5t",
        "Password123",
        "PASSWORD1234!",
        "Senha123456",
        "senha1234!!",
        "Brasil12345",
        "Qwerty123456",
        "Qwerty@2024!",
        "Iloveyou!!!",
        "SuperMan#99",
        "p@ssw0rd123",
        "Corinthians1",
    ] {
        assert_signup_refuses("ana@example.com", common, "password_common").await;
    }
}

#[tokio::test]
async fn signup_refuses_a_password_with_fewer_than_three_character_types() {
    for poor in [
        "1234512345",
        "1234567890",
        "aaaaaaaaaa",
        "abcdefghijkl",
        "ABCDEFGHIJKL",
        "abcdefghij1",
        "ABCDEFGHIJ1",
        "Abcdefghijkl",
        "the quick brown fox jumps",
        "çãõéíóúàê1",
        "ÇÃÕÉÍÓÚÀÊ1",
        "!#$%&/()=?!",
        "1234 5678 90",
    ] {
        assert_signup_refuses("ana@example.com", poor, "password_composition").await;
    }
}

#[tokio::test]
async fn signup_judges_the_composition_before_the_patterns_the_e_mail_and_the_name() {
    // Two types only: told how to fix the mix, not about a pattern, a common
    // password or the e-mail.
    for (password, email) in [
        ("1234512345", "ana@example.com"),
        ("qwertyuiop", "ana@example.com"),
        ("password1234", "ana@example.com"),
        ("brunolima1234", "bruno.lima@example.com"),
        ("souza is my family", "ana@example.com"),
    ] {
        assert_signup_refuses(email, password, "password_composition").await;
    }
}

#[tokio::test]
async fn signup_accepts_three_or_four_character_types_of_any_kind() {
    let app = TestApp::spawn().await;

    for (n, password) in [
        "Zk3mPq9wLx",          // lowercase, uppercase, digit
        "nf8$kd29xq#w",        // lowercase, digit, symbol
        "ZK3MPQ9W!L",          // uppercase, digit, symbol
        "Zk3!mPq9wLx",         // all four
        "ação São 2024!",      // accents, space and a symbol
        "senha—boa—7xy",       // a non-ASCII symbol (em dash)
        "Çãõéíóúàê1",          // accented upper and lower case letters
        "Frase com espaços Z", // lowercase, uppercase, spaces
    ]
    .into_iter()
    .enumerate()
    {
        let response = app
            .session()
            .request_sign_up(&format!("user{n}@example.com"), "Ana Souza", password)
            .await;
        assert_eq!(response.status(), 202, "{password:?} should be accepted");
    }
}

#[tokio::test]
async fn signup_refuses_obvious_sequences_and_repetitions() {
    // Each has at least 3 character types, so the composition rule is met and
    // the sequence rule is the one that fires.
    for obvious in [
        "Aaaaaaaaaa1",
        "Abcdefghij1",
        "Zyxwvutsrqp1!",
        "Ab1Ab1Ab1Ab1",
        "Ab1!Ab1!Ab1!",
        "Abcdeabcde1",
        "Senhasenha1",
        "Qwertyuiop1!",
        "Asdfghjkl;1",
        "Abcdefghij!1",
    ] {
        assert_signup_refuses("ana@example.com", obvious, "password_sequence").await;
    }
}

#[tokio::test]
async fn signup_refuses_a_password_that_contains_the_email_before_the_at_sign() {
    for password in [
        "xk!bruno.lima#92",
        "BRUNO.LIMA-rocks",
        "My-brunolima-pass",
        "bruno_lima_2024!",
        "Bruno.lima@example.com",
        "Prefix+bruno.lima@example.com",
    ] {
        assert_signup_refuses(
            "Bruno.Lima@example.com",
            password,
            "password_contains_email",
        )
        .await;
    }
}

#[tokio::test]
async fn signup_ignores_a_very_short_email_fragment_to_avoid_false_positives() {
    let app = TestApp::spawn().await;

    let response = app
        .session()
        .request_sign_up("ana@example.com", "Ana Souza", "Ana paints Blue cats")
        .await;

    assert_eq!(response.status(), 202);
}

#[tokio::test]
async fn signup_accepts_a_long_passphrase_and_the_largest_size() {
    let app = TestApp::spawn().await;

    let passphrase = app
        .session()
        .request_sign_up(
            "ana@example.com",
            "Ana Souza",
            "The quick brown fox jumps over the lazy dog",
        )
        .await;

    assert_eq!(passphrase.status(), 202);
    // Not a repeated block: that would be refused as a repetition.
    let mut longest = String::new();
    let mut n = 0;
    while longest.chars().count() < 128 {
        longest.push_str(&format!("palavra{n} "));
        n += 1;
    }
    let longest: String = longest.chars().take(128).collect();
    assert_eq!(longest.chars().count(), 128);
    assert_eq!(
        app.session()
            .request_sign_up("cris@example.com", "Cris Melo", &longest)
            .await
            .status(),
        202,
        "128 characters is the largest accepted size"
    );
}

#[tokio::test]
async fn the_signed_up_password_is_the_one_that_logs_in() {
    let app = TestApp::spawn().await;
    app.session()
        .sign_up(
            "ana@example.com",
            "The quick brown fox jumps over the lazy dog",
        )
        .await;

    let response = app
        .session()
        .log_in(
            "ana@example.com",
            "The quick brown fox jumps over the lazy dog",
        )
        .await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn login_with_the_right_password_starts_a_session_even_if_the_email_case_differs() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let browser = app.session();

    let response = browser.log_in("ANA@example.com", PASSWORD).await;

    assert_eq!(response.status(), 200);
    let set_cookie = response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(set_cookie.starts_with(&format!("{SESSION_COOKIE}=")));
    assert!(set_cookie.contains("HttpOnly"));
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["user"]["email"], json!("ana@example.com"));
}

#[tokio::test]
async fn login_fails_the_same_way_for_a_wrong_password_and_an_unknown_email() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let browser = app.session();

    let wrong_password = browser.log_in("ana@example.com", "not the password").await;
    let unknown_email = browser.log_in("nobody@example.com", PASSWORD).await;

    for response in [wrong_password, unknown_email] {
        assert_eq!(response.status(), 401);
        assert!(response.headers().get(SET_COOKIE).is_none());
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], json!("invalid_credentials"));
    }
}

#[tokio::test]
async fn a_protected_route_rejects_a_request_without_a_session() {
    let app = TestApp::spawn().await;
    let visitor = app.session();

    let response = visitor.get("/auth/me").await;

    assert_eq!(response.status(), 401);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("unauthenticated"));
}

#[tokio::test]
async fn a_protected_route_serves_the_user_of_the_session_cookie() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let response = browser.get("/auth/me").await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["user"]["email"], json!("ana@example.com"));
}

#[tokio::test]
async fn a_session_started_by_login_works_on_a_protected_route() {
    let app = TestApp::spawn().await;
    app.session().sign_up("ana@example.com", PASSWORD).await;
    let browser = app.session();
    browser.log_in("ana@example.com", PASSWORD).await;

    let response = browser.get("/auth/me").await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn each_user_only_ever_sees_their_own_account() {
    let app = TestApp::spawn().await;
    let ana = app.session();
    let bruno = app.session();
    ana.sign_up("ana@example.com", PASSWORD).await;
    bruno.sign_up("bruno@example.com", PASSWORD).await;

    let ana_view: Value = ana.get("/auth/me").await.json().await.unwrap();
    let bruno_view: Value = bruno.get("/auth/me").await.json().await.unwrap();

    assert_eq!(ana_view["user"]["email"], json!("ana@example.com"));
    assert_eq!(bruno_view["user"]["email"], json!("bruno@example.com"));
    assert_ne!(ana_view["user"]["id"], bruno_view["user"]["id"]);
}

#[tokio::test]
async fn a_protected_route_rejects_tokens_that_are_forged_expired_or_for_nobody() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    let signup: Value = browser
        .sign_up("ana@example.com", PASSWORD)
        .await
        .json()
        .await
        .unwrap();
    let ana_id = signup["user"]["id"].as_str().unwrap();
    let day = 24 * 60 * 60;

    let rejected = [
        // Signed with another secret.
        forge_token("another-secret-another-secret-12345", ana_id, now() + day),
        // Signed by us, but already expired.
        forge_token(TEST_JWT_SECRET, ana_id, now() - day),
        // Signed by us and current, for a user that does not exist.
        forge_token(
            TEST_JWT_SECRET,
            &uuid::Uuid::new_v4().to_string(),
            now() + day,
        ),
    ];
    for token in rejected {
        let response = browser
            .get_with_cookie("/auth/me", &format!("{SESSION_COOKIE}={token}"))
            .await;
        assert_eq!(response.status(), 401, "token {token} should be rejected");
    }
    let garbage = browser
        .get_with_cookie("/auth/me", &format!("{SESSION_COOKIE}=not-a-jwt"))
        .await;
    assert_eq!(garbage.status(), 401);
}

#[tokio::test]
async fn logout_expires_the_session_cookie() {
    let app = TestApp::spawn().await;
    let browser = app.session();
    browser.sign_up("ana@example.com", PASSWORD).await;

    let response = browser.post_empty("/auth/logout").await;

    assert_eq!(response.status(), 204);
    let set_cookie = response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(set_cookie.starts_with(&format!("{SESSION_COOKIE}=;")));
    assert!(set_cookie.contains("Max-Age=0"));
    assert!(set_cookie.contains("HttpOnly"));
    assert_eq!(browser.get("/auth/me").await.status(), 401);
}

#[tokio::test]
async fn logout_without_a_session_is_harmless() {
    let app = TestApp::spawn().await;

    let response = app.session().post_empty("/auth/logout").await;

    assert_eq!(response.status(), 204);
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// A signed session token, as the API would issue, but with a chosen secret,
/// user and expiry.
fn forge_token(secret: &str, user_id: &str, expires_at: i64) -> String {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &json!({ "sub": user_id, "iat": expires_at - 3600, "exp": expires_at }),
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

#[tokio::test]
async fn signup_rejects_a_body_that_is_not_the_expected_json() {
    let app = TestApp::spawn().await;
    let browser = app.session();

    let response = browser
        .post_json("/auth/signup", &json!({ "email": "ana@example.com" }))
        .await;

    assert_eq!(response.status(), 400);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("invalid_request"));
}
