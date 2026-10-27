//! Shared helpers for integration tests.
//!
//! Tests exercise the real HTTP API against a real PostgreSQL, with "today"
//! under the test's control. Later tickets only call `TestApp::spawn()`.

#![allow(dead_code)]

mod postgres;

use std::sync::{Arc, Mutex};

use chrono::NaiveDate;
use reqwest::Response;
use tabwallet_backend::{
    app::{AppState, build_router},
    clock::Clock,
};
use tokio::net::TcpListener;

/// A clock whose "today" the test moves by hand.
#[derive(Clone)]
pub struct TestClock(Arc<Mutex<NaiveDate>>);

impl Clock for TestClock {
    fn today(&self) -> NaiveDate {
        *self.0.lock().unwrap()
    }
}

pub struct TestApp {
    address: String,
    client: reqwest::Client,
    clock: TestClock,
}

impl TestApp {
    /// Starts the API on a free port against a new database. "Today" starts
    /// at 2026-01-15; use `set_today` to move it.
    pub async fn spawn() -> Self {
        let pool = postgres::fresh_database().await;
        let clock = TestClock(Arc::new(Mutex::new(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
        )));
        let state = AppState {
            pool,
            clock: Arc::new(clock.clone()),
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
