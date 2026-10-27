//! Real PostgreSQL for integration tests.
//!
//! By default one PostgreSQL container (via testcontainers, so it works the
//! same on a laptop and on GitHub Actions) is started per test binary and
//! shared by its tests; every test gets its own freshly migrated database.
//!
//! Set `TEST_DATABASE_URL` to a server where the user may `CREATE DATABASE`
//! to skip the container and use that server instead.

use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};

use sqlx::{Connection, PgConnection, PgPool};
use tabwallet_backend::db;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ImageExt, runners::AsyncRunner},
};

const POSTGRES_TAG: &str = "17-alpine";

static ADMIN_URL: OnceLock<String> = OnceLock::new();
static NEXT_DATABASE: AtomicUsize = AtomicUsize::new(0);

/// Returns a pool connected to a new, empty, fully migrated database.
pub async fn fresh_database() -> PgPool {
    let admin_url = ADMIN_URL
        .get_or_init(|| std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| start_container()))
        .clone();

    let name = format!(
        "test_{}_{}",
        std::process::id(),
        NEXT_DATABASE.fetch_add(1, Ordering::SeqCst)
    );
    let mut admin = PgConnection::connect(&admin_url).await.unwrap();
    // The name is built from the pid and a counter, never from user input.
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&mut admin)
        .await
        .unwrap();

    let pool = db::connect(&with_database(&admin_url, &name))
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    pool
}

/// Starts the container on its own thread and runtime: a container started
/// from a `#[tokio::test]` runtime would die with that test's runtime, while
/// the tests of one binary share it. The container is never dropped here; the
/// testcontainers reaper removes it when the test process exits.
fn start_container() -> String {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (url, _container) = runtime.block_on(async {
            let container = Postgres::default()
                .with_tag(POSTGRES_TAG)
                .start()
                .await
                .expect("failed to start the PostgreSQL test container (is Docker running?)");
            let host = container.get_host().await.unwrap();
            let port = container.get_host_port_ipv4(5432).await.unwrap();
            let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
            (url, container)
        });
        sender.send(url).unwrap();
        // Park forever so the container and the runtime serving its Docker
        // connection stay alive for the whole test process.
        loop {
            std::thread::park();
        }
    });
    receiver.recv().unwrap()
}

fn with_database(url: &str, database: &str) -> String {
    let (base, _) = url.rsplit_once('/').unwrap();
    format!("{base}/{database}")
}
