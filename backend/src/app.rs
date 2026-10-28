use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, patch, post},
};
use sqlx::PgPool;

use crate::{
    auth::{Auth, require_auth},
    clock::Clock,
    handlers,
};

/// Shared state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub clock: Arc<dyn Clock>,
    pub auth: Auth,
}

pub fn build_router(state: AppState) -> Router {
    // Routes that need a signed-in user. Add new ones here: `require_auth`
    // rejects requests without a valid session with 401 and gives the handler
    // an `AuthUser`, whose `id` every query must be scoped to.
    let protected = Router::new()
        .route("/auth/me", get(handlers::auth::me))
        .route(
            "/categories",
            get(handlers::categories::list).post(handlers::categories::create),
        )
        .route("/categories/{id}", patch(handlers::categories::rename))
        .route(
            "/categories/{id}/deactivate",
            post(handlers::categories::deactivate),
        )
        .route(
            "/categories/{id}/reactivate",
            post(handlers::categories::reactivate),
        )
        .route_layer(from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/auth/signup", post(handlers::auth::sign_up))
        .route("/auth/login", post(handlers::auth::log_in))
        .route("/auth/logout", post(handlers::auth::log_out))
        .merge(protected)
        .with_state(state)
}
