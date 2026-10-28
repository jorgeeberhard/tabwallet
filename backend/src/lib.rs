//! TabWallet back-end: an Axum HTTP API over PostgreSQL.
//!
//! Layers (see the project charter, section 4.3):
//! - `handlers`: HTTP routes; validate input and call a use case.
//! - `use_cases`: one `*UseCase` per business operation, unaware of HTTP.
//! - `repositories`: the only code that talks to PostgreSQL (via `sqlx`).
//!
//! Cross-cutting: `auth` (sessions and the `require_auth` middleware), `error`
//! (the stable JSON error shape) and `ids` (typed identifiers).

pub mod app;
pub mod auth;
pub mod clock;
pub mod db;
pub mod error;
pub mod handlers;
pub mod ids;
pub mod repositories;
pub mod use_cases;
