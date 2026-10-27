use axum::{Router, routing::get};
use sqlx::PgPool;

use crate::handlers;

/// Shared state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health::health))
        .with_state(state)
}
