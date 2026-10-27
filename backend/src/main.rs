use std::sync::Arc;

use tabwallet_backend::{
    app::{AppState, build_router},
    clock::SystemClock,
    db,
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // A missing `.env` is fine: production sets real environment variables.
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    let pool = db::connect(&database_url)
        .await
        .expect("failed to connect to the database");
    db::migrate(&pool).await.expect("failed to run migrations");

    let listener = TcpListener::bind(&bind_addr)
        .await
        .expect("failed to bind the listener");
    tracing::info!("listening on {bind_addr}");
    let state = AppState {
        pool,
        clock: Arc::new(SystemClock),
    };
    axum::serve(listener, build_router(state))
        .await
        .expect("server error");
}
