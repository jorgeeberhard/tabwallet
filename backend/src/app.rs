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
    email::EmailSender,
    handlers,
};

/// Shared state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub clock: Arc<dyn Clock>,
    pub auth: Auth,
    /// Sends transactional e-mail (password-reset links).
    pub email: Arc<dyn EmailSender>,
    /// Public address of the front-end, for the links in e-mails.
    pub app_url: String,
}

pub fn build_router(state: AppState) -> Router {
    // Routes that need a signed-in user. Add new ones here: `require_auth`
    // rejects requests without a valid session with 401 and gives the handler
    // an `AuthUser`, whose `id` every query must be scoped to.
    let protected = Router::new()
        .route("/auth/me", get(handlers::auth::me))
        .route("/profile", patch(handlers::profile::update))
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
        .route("/balance", get(handlers::balance::get))
        .route("/balance/opening", post(handlers::balance::set_opening))
        .route(
            "/transactions",
            get(handlers::transactions::list).post(handlers::transactions::create),
        )
        .route(
            "/transactions/{id}",
            patch(handlers::transactions::update).delete(handlers::transactions::delete),
        )
        .route(
            "/credit-cards",
            get(handlers::credit_cards::list).post(handlers::credit_cards::create),
        )
        .route("/credit-cards/{id}", patch(handlers::credit_cards::update))
        .route(
            "/credit-cards/{id}/deactivate",
            post(handlers::credit_cards::deactivate),
        )
        .route(
            "/credit-cards/{id}/reactivate",
            post(handlers::credit_cards::reactivate),
        )
        .route(
            "/credit-cards/{id}/invoices",
            get(handlers::credit_cards::invoices),
        )
        .route(
            "/credit-cards/{id}/invoices/{closing_month}",
            get(handlers::credit_cards::invoice),
        )
        .route("/card-purchases", post(handlers::card_purchases::create))
        .route(
            "/card-purchases/{id}",
            patch(handlers::card_purchases::update).delete(handlers::card_purchases::delete),
        )
        .route_layer(from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/auth/signup", post(handlers::auth::sign_up))
        .route("/auth/login", post(handlers::auth::log_in))
        .route("/auth/logout", post(handlers::auth::log_out))
        .route(
            "/auth/password-reset/request",
            post(handlers::password_reset::request),
        )
        .route(
            "/auth/password-reset/confirm",
            post(handlers::password_reset::confirm),
        )
        .merge(protected)
        .with_state(state)
}
