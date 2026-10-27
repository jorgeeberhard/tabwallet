use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};

use crate::{app::AppState, repositories::health::HealthRepository, use_cases::CheckHealthUseCase};

pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let use_case = CheckHealthUseCase::new(HealthRepository::new(state.pool));
    let report = use_case.execute().await;

    let (status_code, status, database) = if report.database_up {
        (StatusCode::OK, "ok", "up")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "degraded", "down")
    };
    (status_code, Json(json!({ "status": status, "database": database })))
}
