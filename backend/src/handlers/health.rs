use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};

use crate::{app::AppState, repositories::health::HealthRepository, use_cases::CheckHealthUseCase};

/// Liveness: the process is up and serving. It deliberately does not touch the
/// database (and has no use case: there is no business rule), so a probe that
/// runs every few seconds cannot keep a serverless PostgreSQL awake. Use
/// `/health` to know whether the database answers too.
pub async fn live() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let use_case = CheckHealthUseCase::new(HealthRepository::new(state.pool), state.clock);
    let report = use_case.execute().await;

    let (status_code, status, database) = if report.database_up {
        (StatusCode::OK, "ok", "up")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "degraded", "down")
    };
    let body = json!({
        "status": status,
        "database": database,
        "today": report.today.to_string(),
    });
    (status_code, Json(body))
}
