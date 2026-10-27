//! TabWallet back-end: an Axum HTTP API over PostgreSQL.
//!
//! Layers (see the project charter, section 4.3):
//! - `handlers`: HTTP routes; validate input and call a use case.
//! - `use_cases`: one `*UseCase` per business operation, unaware of HTTP.
//! - `repositories`: the only code that talks to PostgreSQL (via `sqlx`).

pub mod app;
pub mod db;
pub mod handlers;
pub mod repositories;
pub mod use_cases;
