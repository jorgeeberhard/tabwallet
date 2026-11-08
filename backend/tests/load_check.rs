//! Load check for the performance requirement: 95% of API requests answer in
//! up to 300 ms under normal load (project charter, section 3).
//!
//! It is `#[ignore]`d, so `cargo test` and the CI skip it. Run it with
//! `backend/scripts/load-check.sh` (release build, throwaway PostgreSQL in
//! Docker), which prints the table of percentiles.
//!
//! It seeds one realistic user through the HTTP API (about 2,000 transactions
//! over two years, fixed transactions, four cards with purchases and
//! installment purchases, budgets), then calls the main read endpoints and
//! reports p50, p95 and max per endpoint, first one request at a time and then
//! with several clients at once. The server runs in this process and the
//! client talks to it over loopback HTTP, so the numbers cover routing,
//! authentication, use cases and PostgreSQL, but not TLS or the Next.js proxy.

mod common;

use std::{sync::Arc, time::Instant};

use chrono::{Datelike, Days, Months, NaiveDate};
use common::{Session, TestApp};
use serde_json::{Value, json};
use tokio::task::JoinSet;

const PASSWORD: &str = "correct horse battery";

/// The requirement, in milliseconds. `LOAD_CHECK_P95_MS` overrides it.
const DEFAULT_P95_LIMIT_MS: f64 = 300.0;

/// Seed sizes. `LOAD_CHECK_TRANSACTIONS` overrides the number of transactions.
const DEFAULT_TRANSACTIONS: usize = 2_000;
const HISTORY_MONTHS: u32 = 24;
/// Per step of the history (three steps a month).
const CARD_PURCHASES_PER_STEP: usize = 6;
const INSTALLMENT_PURCHASES_PER_STEP: usize = 1;

/// Requests per endpoint in each phase.
const SEQUENTIAL_REQUESTS: usize = 100;
const CONCURRENT_CLIENTS: usize = 8;
const REQUESTS_PER_CLIENT: usize = 30;

/// A small deterministic generator, so every run seeds the same data.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % below
    }
}

fn env_number<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

async fn category_ids(session: &Session, kind: &str) -> Vec<String> {
    let body: Value = session.get("/categories").await.json().await.unwrap();
    body["categories"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|category| category["type"] == json!(kind))
        .filter(|category| category["name"] != json!("Juros e encargos"))
        .map(|category| category["id"].as_str().unwrap().to_string())
        .collect()
}

/// Sends every body to `path`, `CONCURRENT_CLIENTS` at a time, and checks that
/// each answered 201.
async fn post_all(session: &Arc<Session>, path: &'static str, bodies: Vec<Value>) -> Vec<Value> {
    let mut created = Vec::new();
    let mut pending = bodies.into_iter();
    let mut running = JoinSet::new();
    loop {
        while running.len() < CONCURRENT_CLIENTS {
            let Some(body) = pending.next() else { break };
            let session = session.clone();
            running.spawn(async move {
                let response = session.post_json(path, &body).await;
                let status = response.status();
                let text = response.text().await.unwrap();
                assert_eq!(status, 201, "POST {path} {body}: {text}");
                serde_json::from_str::<Value>(&text).unwrap()
            });
        }
        match running.join_next().await {
            Some(result) => created.push(result.unwrap()),
            None => break,
        }
    }
    created
}

struct Seeded {
    session: Arc<Session>,
    first_card: String,
    /// `YYYY-MM` closing month of an invoice of the first card with purchases.
    busy_invoice_month: String,
    transactions: usize,
    card_purchases: usize,
    installment_purchases: usize,
}

async fn seed(app: &TestApp, transactions: usize) -> Seeded {
    let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
    let mut random = Lcg(42);
    let session = Arc::new(app.session());
    let response = session.sign_up("load@example.com", PASSWORD).await;
    assert_eq!(response.status(), 201);

    let response = session
        .post_json("/balance/opening", &json!({ "amount_cents": 1_500_000 }))
        .await;
    assert_eq!(response.status(), 201);

    let expense_categories = category_ids(&session, "expense").await;
    let income_categories = category_ids(&session, "income").await;
    let pick =
        |random: &mut Lcg, ids: &[String]| ids[random.next(ids.len() as u64) as usize].clone();
    // From 720 days back to 80 days ahead: history plus Projections.
    let some_day =
        |random: &mut Lcg| (today - Days::new(720) + Days::new(random.next(800))).to_string();

    let bodies = (0..transactions)
        .map(|index| {
            let is_income = index % 8 == 0;
            let (kind, ids) = if is_income {
                ("income", &income_categories)
            } else {
                ("expense", &expense_categories)
            };
            json!({
                "type": kind,
                "amount_cents": 500 + random.next(60_000) as i64,
                "date": some_day(&mut random),
                "description": format!("lançamento {index}"),
                "category_id": pick(&mut random, ids),
            })
        })
        .collect();
    post_all(&session, "/transactions", bodies).await;

    let fixed = [
        ("income", 650_000, 5, "salário"),
        ("expense", 180_000, 10, "aluguel"),
        ("expense", 32_000, 15, "condomínio"),
        ("expense", 9_990, 20, "internet"),
        ("expense", 12_000, 28, "academia"),
        ("expense", 5_500, 31, "streaming"),
    ]
    .into_iter()
    .map(|(kind, amount_cents, day_of_month, description)| {
        let ids = if kind == "income" {
            &income_categories
        } else {
            &expense_categories
        };
        json!({
            "type": kind,
            "amount_cents": amount_cents,
            "day_of_month": day_of_month,
            "description": description,
            "category_id": ids[0],
        })
    })
    .collect();
    post_all(&session, "/recurring-transactions", fixed).await;

    // The cards exist since the start of the history below.
    app.set_today(2024, 1, 15);
    let cards = [("Roxinho", 5, 12), ("Azul", 10, 17), ("Preto", 20, 27), ("Loja", 28, 5)]
        .into_iter()
        .map(|(name, closing_day, due_day)| {
            json!({ "name": name, "closing_day": closing_day, "due_day": due_day })
        })
        .collect();
    // Requests finish in any order; sort by name so every run seeds the same.
    let mut cards: Vec<(String, String)> = post_all(&session, "/credit-cards", cards)
        .await
        .iter()
        .map(|body| {
            let card = &body["credit_card"];
            (
                card["name"].as_str().unwrap().to_string(),
                card["id"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    cards.sort();
    let cards: Vec<String> = cards.into_iter().map(|(_, id)| id).collect();

    // A paid invoice takes no new purchase, so the history is built the way a
    // real user builds it: "today" moves through the 1st, 11th and 21st of every
    // month and each step launches purchases dated up to 7 days back (so that in
    // February they still fall after the closing day 20). With the closing days
    // above, those always fall in unpaid invoices, whatever card takes them.
    let mut card_purchases = 0;
    let mut installment_purchases = 0;
    for step in 0..HISTORY_MONTHS * 3 {
        let first_of_month = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap() + Months::new(step / 3);
        let step_today = first_of_month + Days::new(10 * u64::from(step % 3));
        app.set_today(step_today.year(), step_today.month(), step_today.day());
        let recent_day = |random: &mut Lcg| (step_today - Days::new(random.next(8))).to_string();
        let month_index = step;

        let purchases = (0..CARD_PURCHASES_PER_STEP)
            .map(|index| {
                json!({
                    "credit_card_id": cards[index % cards.len()],
                    "amount_cents": 800 + random.next(40_000) as i64,
                    "date": recent_day(&mut random),
                    "description": format!("compra {month_index}-{index}"),
                    "category_id": pick(&mut random, &expense_categories),
                })
            })
            .collect();
        card_purchases += post_all(&session, "/card-purchases", purchases).await.len();

        let installments = (0..INSTALLMENT_PURCHASES_PER_STEP)
            .map(|index| {
                json!({
                    "credit_card_id": cards[index % cards.len()],
                    "total_cents": 30_000 + random.next(300_000) as i64,
                    "installment_count": 2 + random.next(11) as i64,
                    "date": recent_day(&mut random),
                    "description": format!("parcelado {month_index}-{index}"),
                    "category_id": pick(&mut random, &expense_categories),
                })
            })
            .collect();
        installment_purchases += post_all(&session, "/installment-purchases", installments)
            .await
            .len();
    }
    app.set_today(2026, 1, 15);

    for id in expense_categories.iter().take(6) {
        let response = session
            .put_json(
                &format!("/budgets/{id}"),
                &json!({ "limit_cents": 300_000 }),
            )
            .await;
        assert!(
            response.status().is_success(),
            "budget: {}",
            response.status()
        );
    }

    let first_card = cards[0].clone();
    let invoices: Value = session
        .get(&format!("/credit-cards/{first_card}/invoices"))
        .await
        .json()
        .await
        .unwrap();
    let busy_invoice_month = invoices["invoices"]
        .as_array()
        .unwrap()
        .iter()
        .max_by_key(|invoice| invoice["total_cents"].as_i64().unwrap_or(0))
        .map(|invoice| invoice["closing_month"].as_str().unwrap().to_string())
        .expect("the first card has invoices");

    Seeded {
        session,
        first_card,
        busy_invoice_month,
        transactions,
        card_purchases,
        installment_purchases,
    }
}

/// One thing to call: a label and a GET path (or a login).
struct Probe {
    label: &'static str,
    request: Request,
}

enum Request {
    Get(String),
    LogIn,
}

fn probes(seeded: &Seeded) -> Vec<Probe> {
    let card = &seeded.first_card;
    let get = |label, path: &str| Probe {
        label,
        request: Request::Get(path.to_string()),
    };
    vec![
        get("GET /auth/me", "/auth/me"),
        get("GET /categories", "/categories"),
        get("GET /transactions (page of 50)", "/transactions"),
        get(
            "GET /transactions (one month)",
            "/transactions?from=2025-06-01&to=2025-06-30&limit=200",
        ),
        get(
            "GET /transactions (deep page)",
            "/transactions?limit=50&offset=1500",
        ),
        get(
            "GET /transactions (by card)",
            &format!("/transactions?credit_card_id={card}"),
        ),
        get("GET /balance", "/balance"),
        get(
            "GET /balance/history (24 months)",
            "/balance/history?from=2024-02-01&to=2026-01-15&granularity=month",
        ),
        get(
            "GET /balance/history (365 days)",
            "/balance/history?from=2025-01-16&to=2026-01-15&granularity=day",
        ),
        get("GET /balance/projection (12 months)", "/balance/projection"),
        get(
            "GET /balance/projection (daily)",
            "/balance/projection?to=2026-12-31&granularity=day",
        ),
        get("GET /credit-cards", "/credit-cards"),
        get(
            "GET /credit-cards/{id}/invoices",
            &format!("/credit-cards/{card}/invoices"),
        ),
        get(
            "GET /credit-cards/{id}/invoices/{month}",
            &format!(
                "/credit-cards/{card}/invoices/{}",
                seeded.busy_invoice_month
            ),
        ),
        get(
            "GET /credit-cards/{id}/anticipable-installments",
            &format!("/credit-cards/{card}/anticipable-installments"),
        ),
        get("GET /recurring-transactions", "/recurring-transactions"),
        get("GET /budgets/status", "/budgets/status"),
        get(
            "GET /reports/expenses-by-category",
            "/reports/expenses-by-category?from=2024-02&to=2026-01",
        ),
        get(
            "GET /reports/income-vs-expenses",
            "/reports/income-vs-expenses?from=2024-02&to=2026-01",
        ),
        Probe {
            label: "POST /auth/login (Argon2)",
            request: Request::LogIn,
        },
    ]
}

async fn run(session: &Session, request: &Request) -> f64 {
    let started = Instant::now();
    let response = match request {
        Request::Get(path) => session.get(path).await,
        Request::LogIn => session.log_in("load@example.com", PASSWORD).await,
    };
    let status = response.status();
    let body = response.bytes().await.unwrap();
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    assert!(
        status.is_success(),
        "{}: {}",
        status,
        String::from_utf8_lossy(&body)
    );
    elapsed
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let rank = ((sorted.len() as f64) * fraction).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

struct Row {
    label: &'static str,
    p50: f64,
    p95: f64,
    max: f64,
    samples: Vec<f64>,
}

fn summarize(label: &'static str, mut samples: Vec<f64>) -> Row {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Row {
        label,
        p50: percentile(&samples, 0.50),
        p95: percentile(&samples, 0.95),
        max: *samples.last().unwrap(),
        samples,
    }
}

fn print_table(title: &str, rows: &[Row]) {
    println!("\n{title}");
    println!(
        "{:<52} {:>9} {:>9} {:>9}",
        "endpoint", "p50 ms", "p95 ms", "max ms"
    );
    for row in rows {
        println!(
            "{:<52} {:>9.1} {:>9.1} {:>9.1}",
            row.label, row.p50, row.p95, row.max
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "load check: run it with backend/scripts/load-check.sh"]
async fn the_main_flows_answer_in_300_ms_at_the_95th_percentile() {
    let limit_ms = env_number("LOAD_CHECK_P95_MS", DEFAULT_P95_LIMIT_MS);
    let transactions = env_number("LOAD_CHECK_TRANSACTIONS", DEFAULT_TRANSACTIONS);

    let app = TestApp::spawn().await;
    let started = Instant::now();
    let seeded = seed(&app, transactions).await;
    println!(
        "seeded 1 user: {} transactions, 6 fixed transactions, 4 cards with {} purchases \
         and {} installment purchases, 6 budgets, in {:.1} s",
        seeded.transactions,
        seeded.card_purchases,
        seeded.installment_purchases,
        started.elapsed().as_secs_f64()
    );

    let probes = probes(&seeded);

    // Warm-up: connection pool, caches, query plans.
    for probe in &probes {
        run(&seeded.session, &probe.request).await;
    }

    // Phase 1: one client, one request at a time.
    let mut sequential = Vec::new();
    for probe in &probes {
        let mut samples = Vec::new();
        for _ in 0..SEQUENTIAL_REQUESTS {
            samples.push(run(&seeded.session, &probe.request).await);
        }
        sequential.push(summarize(probe.label, samples));
    }
    print_table(
        &format!("One client, {SEQUENTIAL_REQUESTS} requests per endpoint"),
        &sequential,
    );

    // Phase 2: several clients at once, each looping over every endpoint.
    let probes = Arc::new(probes);
    let mut clients = JoinSet::new();
    for _ in 0..CONCURRENT_CLIENTS {
        let session = seeded.session.clone();
        let probes = probes.clone();
        clients.spawn(async move {
            let mut samples = vec![Vec::new(); probes.len()];
            for _ in 0..REQUESTS_PER_CLIENT {
                for (index, probe) in probes.iter().enumerate() {
                    samples[index].push(run(&session, &probe.request).await);
                }
            }
            samples
        });
    }
    let mut merged = vec![Vec::new(); probes.len()];
    while let Some(result) = clients.join_next().await {
        for (index, samples) in result.unwrap().into_iter().enumerate() {
            merged[index].extend(samples);
        }
    }
    let concurrent: Vec<Row> = probes
        .iter()
        .zip(merged)
        .map(|(probe, samples)| summarize(probe.label, samples))
        .collect();
    print_table(
        &format!(
            "{CONCURRENT_CLIENTS} clients at once, {REQUESTS_PER_CLIENT} requests per client and endpoint"
        ),
        &concurrent,
    );

    // The requirement is about all requests of the main flows together.
    for (title, rows) in [
        ("one client", &sequential),
        ("several clients", &concurrent),
    ] {
        let mut all: Vec<f64> = rows
            .iter()
            .filter(|row| !row.label.starts_with("POST"))
            .flat_map(|row| row.samples.iter().copied())
            .collect();
        all.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "\nAll GET requests, {title}: {} requests, p50 {:.1} ms, p95 {:.1} ms, p99 {:.1} ms",
            all.len(),
            percentile(&all, 0.50),
            percentile(&all, 0.95),
            percentile(&all, 0.99)
        );
    }

    let over: Vec<String> = sequential
        .iter()
        .chain(&concurrent)
        .filter(|row| row.p95 > limit_ms)
        .map(|row| format!("{} (p95 {:.1} ms)", row.label, row.p95))
        .collect();
    assert!(
        over.is_empty(),
        "p95 above {limit_ms} ms: {}",
        over.join(", ")
    );
}
