//! Shared helpers for integration tests.
//!
//! Tests exercise the real HTTP API against a real PostgreSQL. Later tickets
//! only call `TestApp::spawn()`.

#![allow(dead_code)]

mod postgres;

use reqwest::Response;
use tabwallet_backend::app::{AppState, build_router};
use tokio::net::TcpListener;

pub struct TestApp {
    address: String,
    client: reqwest::Client,
}

impl TestApp {
    pub async fn spawn() -> Self {
        let pool = postgres::fresh_database().await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            axum::serve(listener, build_router(AppState { pool }))
                .await
                .unwrap();
        });
        Self {
            address,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get(&self, path: &str) -> Response {
        self.client
            .get(format!("{}{}", self.address, path))
            .send()
            .await
            .unwrap()
    }
}
