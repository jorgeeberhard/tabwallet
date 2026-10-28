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

/// The address the test API puts in e-mail links.
pub const TEST_APP_URL: &str = "http://app.test";

pub struct TestApp {
    address: String,
    client: reqwest::Client,
    clock: TestClock,
    emails: RecordingEmailSender,
}

/// A browser-like client: it remembers the session cookie the API sets and
/// sends it back, so a test can sign up, log in and call protected routes the
/// way the front-end does. Each session is a separate "browser".
pub struct Session {
    address: String,
    client: reqwest::Client,
    cookie: Mutex<Option<String>>,
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

    pub async fn patch_json(&self, path: &str, body: &Value) -> Response {
        self.send(
            self.client
                .patch(format!("{}{}", self.address, path))
                .json(body),
        )
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

    /// Registers a user and, as the API does, leaves this session signed in.
    pub async fn sign_up(&self, email: &str, password: &str) -> Response {
        self.post_json(
            "/auth/signup",
            &json!({ "email": email, "password": password }),
        )
        .await
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
            pool,
            clock: Arc::new(clock.clone()),
            // Plain HTTP in tests, so the cookie cannot be `Secure`.
            auth: Auth::new(TEST_JWT_SECRET, false).unwrap(),
            email: Arc::new(emails.clone()),
            app_url: TEST_APP_URL.to_string(),
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
