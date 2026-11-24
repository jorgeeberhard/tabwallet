//! Shared helpers for integration tests.
//!
//! Tests exercise the real HTTP API against a real PostgreSQL, with "today"
//! under the test's control. Later tickets only call `TestApp::spawn()`, then
//! `app.session()` for a browser-like client, and `session.sign_up(...)` to
//! get a signed-in user.

#![allow(dead_code)]

mod postgres;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::{
    Response,
    header::{COOKIE, SET_COOKIE},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tabwallet_backend::{
    app::{AppState, build_router},
    auth::Auth,
    clock::Clock,
    email::{EmailError, EmailMessage, EmailSender},
};
use tokio::net::TcpListener;

pub use tabwallet_backend::auth::SESSION_COOKIE;

/// The JWT secret the test API signs with; tests use it to forge tokens.
pub const TEST_JWT_SECRET: &str = "test-secret-test-secret-test-secret-0123";

/// A clock whose "today" the test moves by hand. `now` is the real instant
/// plus a shift the test sets with `TestApp::advance_time`.
#[derive(Clone)]
pub struct TestClock(Arc<Mutex<NaiveDate>>, Arc<Mutex<chrono::Duration>>);

impl Clock for TestClock {
    fn today(&self) -> NaiveDate {
        *self.0.lock().unwrap()
    }

    fn now(&self) -> DateTime<Utc> {
        Utc::now() + *self.1.lock().unwrap()
    }
}

/// An e-mail sender that sends nothing and remembers every message.
#[derive(Clone, Default)]
pub struct RecordingEmailSender(Arc<Mutex<Vec<EmailMessage>>>);

#[async_trait]
impl EmailSender for RecordingEmailSender {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError> {
        self.0.lock().unwrap().push(message);
        Ok(())
    }
}

/// The name `Session::sign_up` gives the people it creates.
pub const DEFAULT_NAME: &str = "Maria Silva";

/// The address the test API puts in e-mail links.
pub const TEST_APP_URL: &str = "http://app.test";

pub struct TestApp {
    address: String,
    client: reqwest::Client,
    clock: TestClock,
    emails: RecordingEmailSender,
    /// The API's database, for tests that call a repository directly.
    pub pool: PgPool,
}

/// A browser-like client: it remembers the session cookie the API sets and
/// sends it back, so a test can sign up, log in and call protected routes the
/// way the front-end does. Each session is a separate "browser".
pub struct Session {
    address: String,
    client: reqwest::Client,
    cookie: Mutex<Option<String>>,
    emails: RecordingEmailSender,
}

impl Session {
    pub async fn get(&self, path: &str) -> Response {
        self.send(self.client.get(format!("{}{}", self.address, path)))
            .await
    }

    pub async fn post_json(&self, path: &str, body: &Value) -> Response {
        self.send(
            self.client
                .post(format!("{}{}", self.address, path))
                .json(body),
        )
        .await
    }

    /// A POST with one extra request header, e.g. `Origin` or `Referer`.
    pub async fn post_json_with_header(
        &self,
        path: &str,
        body: &Value,
        name: &str,
        value: &str,
    ) -> Response {
        self.send(
            self.client
                .post(format!("{}{}", self.address, path))
                .header(name, value)
                .json(body),
        )
        .await
    }

    /// A GET with one extra request header.
    pub async fn get_with_header(&self, path: &str, name: &str, value: &str) -> Response {
        self.send(
            self.client
                .get(format!("{}{}", self.address, path))
                .header(name, value),
        )
        .await
    }

    pub async fn put_json(&self, path: &str, body: &Value) -> Response {
        self.send(
            self.client
                .put(format!("{}{}", self.address, path))
                .json(body),
        )
        .await
    }

    pub async fn patch_json(&self, path: &str, body: &Value) -> Response {
        self.send(
            self.client
                .patch(format!("{}{}", self.address, path))
                .json(body),
        )
        .await
    }

    pub async fn delete(&self, path: &str) -> Response {
        self.send(self.client.delete(format!("{}{}", self.address, path)))
            .await
    }

    pub async fn post_empty(&self, path: &str) -> Response {
        self.send(self.client.post(format!("{}{}", self.address, path)))
            .await
    }

    /// Sends a GET with exactly this `Cookie` header, ignoring the session's
    /// own cookie. For forged, expired or foreign tokens.
    pub async fn get_with_cookie(&self, path: &str, cookie: &str) -> Response {
        self.client
            .get(format!("{}{}", self.address, path))
            .header(COOKIE, cookie)
            .send()
            .await
            .unwrap()
    }

    /// Gives a test a signed-in user, through the real flow: asks for the
    /// sign-up, opens the link of the confirmation e-mail the fake recorded,
    /// then logs in. The API never signs anyone in at sign-up, so this is
    /// what a person does. Returns the login response (`200` and the user).
    pub async fn sign_up(&self, email: &str, password: &str) -> Response {
        let requested = self.request_sign_up(email, DEFAULT_NAME, password).await;
        assert_eq!(requested.status(), 202, "sign-up of {email} was refused");
        let token = self.latest_confirmation_token(email);
        let confirmed = self.confirm_sign_up(&token).await;
        assert_eq!(confirmed.status(), 204, "confirmation of {email} failed");
        self.log_in(email, password).await
    }

    /// Asks for a sign-up the way the sign-up form does. It only creates a
    /// Pending signup: nobody is signed in and no Account exists yet.
    pub async fn request_sign_up(&self, email: &str, name: &str, password: &str) -> Response {
        self.post_json(
            "/auth/signup",
            &json!({ "email": email, "name": name, "password": password }),
        )
        .await
    }

    /// Opens a confirmation link: the token is the one in the e-mailed link.
    pub async fn confirm_sign_up(&self, token: &str) -> Response {
        self.post_json("/auth/signup/confirm", &json!({ "token": token }))
            .await
    }

    /// Asks for the confirmation e-mail to be sent again.
    pub async fn resend_confirmation(&self, email: &str) -> Response {
        self.post_json("/auth/signup/resend", &json!({ "email": email }))
            .await
    }

    /// Asks to change the e-mail the way the profile form does: only the
    /// e-mail and the current password go in the body. It only creates a
    /// Pending e-mail change.
    pub async fn request_email_change(&self, new_email: &str, current_password: &str) -> Response {
        self.patch_json(
            "/profile",
            &json!({ "email": new_email, "current_password": current_password }),
        )
        .await
    }

    /// Opens the link of a Pending e-mail change: the token is the one in the
    /// e-mailed link. Works with or without a session.
    pub async fn confirm_email_change(&self, token: &str) -> Response {
        self.post_json("/auth/email-change/confirm", &json!({ "token": token }))
            .await
    }

    /// Asks for the link of the Pending e-mail change to be sent again.
    pub async fn resend_email_change(&self) -> Response {
        self.post_empty("/profile/email-change/resend").await
    }

    /// The confirmation token in the latest e-mail sent to `email`.
    pub fn latest_confirmation_token(&self, email: &str) -> String {
        self.latest_token(email, "confirm-email")
    }

    /// The token of the e-mail-change link in the latest e-mail sent to `email`.
    pub fn latest_email_change_token(&self, email: &str) -> String {
        self.latest_token(email, "confirm-email-change")
    }

    fn latest_token(&self, email: &str, page: &str) -> String {
        let link = format!("{TEST_APP_URL}/{page}?token=");
        let message = self
            .emails
            .0
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|message| message.to.eq_ignore_ascii_case(email) && message.text.contains(&link))
            .cloned()
            .unwrap_or_else(|| panic!("no e-mail with a {page} link was sent to {email}"));
        let rest = message.text.split_once(&link).unwrap().1;
        rest.split(char::is_whitespace).next().unwrap().to_string()
    }

    pub async fn log_in(&self, email: &str, password: &str) -> Response {
        self.post_json(
            "/auth/login",
            &json!({ "email": email, "password": password }),
        )
        .await
    }

    /// The `name=value` pair of the session cookie, if the session has one.
    pub fn cookie(&self) -> Option<String> {
        self.cookie.lock().unwrap().clone()
    }

    async fn send(&self, mut request: reqwest::RequestBuilder) -> Response {
        if let Some(cookie) = self.cookie() {
            request = request.header(COOKIE, cookie);
        }
        let response = request.send().await.unwrap();
        self.remember_cookie(&response);
        response
    }

    /// Mimics a browser cookie jar for the session cookie: keep what the API
    /// sets, forget it when the API expires it.
    fn remember_cookie(&self, response: &Response) {
        for header in response.headers().get_all(SET_COOKIE) {
            let header = header.to_str().unwrap();
            let pair = header.split(';').next().unwrap();
            if let Some((name, value)) = pair.split_once('=')
                && name == SESSION_COOKIE
            {
                *self.cookie.lock().unwrap() = if value.is_empty() {
                    None
                } else {
                    Some(pair.to_string())
                };
            }
        }
    }
}

impl Drop for TestApp {
    /// Drops the test's database when the test ends (see
    /// `postgres::drop_database`).
    fn drop(&mut self) {
        if let Some(name) = self.pool.connect_options().get_database() {
            postgres::drop_database(name);
        }
    }
}

impl TestApp {
    /// Starts the API on a free port against a new database. "Today" starts
    /// at 2026-01-15; use `set_today` to move it.
    pub async fn spawn() -> Self {
        Self::spawn_on(postgres::fresh_database().await).await
    }

    /// A new empty database with no migrations applied; see `spawn_on`.
    pub async fn empty_database() -> PgPool {
        postgres::empty_database().await
    }

    /// Starts the API on a database the test prepared itself.
    pub async fn spawn_on(pool: PgPool) -> Self {
        let clock = TestClock(
            Arc::new(Mutex::new(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap())),
            Arc::new(Mutex::new(chrono::Duration::zero())),
        );
        let emails = RecordingEmailSender::default();
        let state = AppState {
            pool: pool.clone(),
            clock: Arc::new(clock.clone()),
            // Plain HTTP in tests, so the cookie cannot be `Secure`.
            auth: Auth::new(TEST_JWT_SECRET, false).unwrap(),
            email: Arc::new(emails.clone()),
            app_url: TEST_APP_URL.to_string(),
            rate_limits: Default::default(),
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            axum::serve(listener, build_router(state)).await.unwrap();
        });
        Self {
            address,
            client: reqwest::Client::new(),
            clock,
            emails,
            pool,
        }
    }

    /// Moves the instant the API sees as "now" forward (for links that
    /// expire after some minutes). It does not change "today".
    pub fn advance_time(&self, by: chrono::Duration) {
        *self.clock.1.lock().unwrap() += by;
    }

    /// Every e-mail the API tried to send, oldest first.
    pub fn sent_emails(&self) -> Vec<EmailMessage> {
        self.emails.0.lock().unwrap().clone()
    }

    /// A new "browser" with no cookies.
    pub fn session(&self) -> Session {
        Session {
            address: self.address.clone(),
            client: reqwest::Client::new(),
            cookie: Mutex::new(None),
            emails: self.emails.clone(),
        }
    }

    /// Sets what the API considers "today" (a date in America/Sao_Paulo).
    pub fn set_today(&self, year: i32, month: u32, day: u32) {
        *self.clock.0.lock().unwrap() = NaiveDate::from_ymd_opt(year, month, day).unwrap();
    }

    pub async fn get(&self, path: &str) -> Response {
        self.client
            .get(format!("{}{}", self.address, path))
            .send()
            .await
            .unwrap()
    }
}
