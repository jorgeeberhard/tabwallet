mod common;

use common::TestApp;
use serde_json::{Value, json};

#[tokio::test]
async fn health_reports_ok_when_database_is_reachable() {
    let app = TestApp::spawn().await;

    let response = app.get("/health").await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["status"], json!("ok"));
    assert_eq!(body["database"], json!("up"));
}

#[tokio::test]
async fn health_reports_today_from_the_injected_clock() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 3, 15);

    let body: Value = app.get("/health").await.json().await.unwrap();
    assert_eq!(body["today"], json!("2026-03-15"));

    app.set_today(2026, 3, 16);

    let body: Value = app.get("/health").await.json().await.unwrap();
    assert_eq!(body["today"], json!("2026-03-16"));
}

/// The orchestrator's liveness probe runs every few seconds: it must say the
/// process is up without touching the database, or it keeps a serverless
/// PostgreSQL awake around the clock.
#[tokio::test]
async fn live_answers_even_when_the_database_is_unreachable() {
    // Nothing listens on port 1, and the pool connects only when used.
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .unwrap();
    let app = TestApp::spawn_on(unreachable).await;

    let live = app.get("/live").await;
    assert_eq!(live.status(), 200);
    let body: Value = live.json().await.unwrap();
    assert_eq!(body, json!({ "status": "ok" }));

    // `/health` still reports the database problem.
    assert_eq!(app.get("/health").await.status(), 503);
}
