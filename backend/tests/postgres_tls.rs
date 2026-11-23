//! The API must reach a managed PostgreSQL (Neon) that demands TLS, so the
//! `sqlx` build has a rustls feature and `sslmode=require` has to work.
//!
//! Needs a PostgreSQL with `ssl=on` and a self-signed certificate, given as
//! `TEST_TLS_DATABASE_URL` (without `sslmode`). `backend/scripts/check.sh` and
//! the CI workflow start one; without the variable the tests skip themselves,
//! because the plain test database cannot speak TLS.

use sqlx::{PgPool, Row};
use tabwallet_backend::db;

fn tls_database_url() -> Option<String> {
    let url = std::env::var("TEST_TLS_DATABASE_URL").ok();
    if url.is_none() {
        eprintln!("TEST_TLS_DATABASE_URL is not set: skipping the PostgreSQL TLS test");
    }
    url
}

/// Whether the server reports this very connection as encrypted.
async fn session_uses_tls(pool: &PgPool) -> bool {
    sqlx::query("SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()")
        .fetch_one(pool)
        .await
        .unwrap()
        .get("ssl")
}

#[tokio::test]
async fn connects_and_migrates_with_sslmode_require() {
    let Some(url) = tls_database_url() else {
        return;
    };

    let pool = db::connect(&format!("{url}?sslmode=require"))
        .await
        .expect("sslmode=require must connect to a server with TLS");

    assert!(
        session_uses_tls(&pool).await,
        "the connection is not encrypted"
    );
    // What `main` does at startup, over the encrypted connection.
    db::migrate(&pool).await.unwrap();
}

#[tokio::test]
async fn still_connects_in_plain_text_with_sslmode_disable() {
    let Some(url) = tls_database_url() else {
        return;
    };

    let pool = db::connect(&format!("{url}?sslmode=disable"))
        .await
        .unwrap();

    assert!(!session_uses_tls(&pool).await);
}
