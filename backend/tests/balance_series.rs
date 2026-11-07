//! The balance history and the Projected Balance (UC10), through the HTTP API.
//! Each test moves "today" with `set_today` and reads the series again.

mod common;

use common::{Session, TestApp};
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";

async fn signed_up(app: &TestApp, email: &str) -> Session {
    let browser = app.session();
    let response = browser.sign_up(email, PASSWORD).await;
    assert_eq!(response.status(), 201);
    browser
}

async fn category_id(browser: &Session, name: &str, kind: &str) -> String {
    let body: Value = browser.get("/categories").await.json().await.unwrap();
    body["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|category| category["name"] == json!(name) && category["type"] == json!(kind))
        .unwrap_or_else(|| panic!("no {kind} category named {name}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Records a transaction in a default category of the same type.
async fn record(browser: &Session, kind: &str, amount_cents: i64, date: &str) {
    let name = if kind == "income" {
        "Salário"
    } else {
        "Outros"
    };
    let response = browser
        .post_json(
            "/transactions",
            &json!({
                "type": kind,
                "amount_cents": amount_cents,
                "date": date,
                "description": "test",
                "category_id": category_id(browser, name, kind).await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201);
}

async fn set_opening(browser: &Session, amount_cents: i64) {
    let response = browser
        .post_json("/balance/opening", &json!({ "amount_cents": amount_cents }))
        .await;
    assert_eq!(response.status(), 201);
}

/// Creates a recurring Transaction (expense in "Moradia", income in
/// "Salário") that starts in the current month.
async fn create_recurring(browser: &Session, kind: &str, amount_cents: i64, day_of_month: u32) {
    let name = if kind == "income" {
        "Salário"
    } else {
        "Moradia"
    };
    let response = browser
        .post_json(
            "/recurring-transactions",
            &json!({
                "type": kind,
                "amount_cents": amount_cents,
                "day_of_month": day_of_month,
                "description": "fixed",
                "category_id": category_id(browser, name, kind).await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201);
}

/// `(date, balance_cents)` of each point of a series response.
fn points(body: &Value) -> Vec<(String, i64)> {
    body["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|point| {
            (
                point["date"].as_str().unwrap().to_string(),
                point["balance_cents"].as_i64().unwrap(),
            )
        })
        .collect()
}

fn expected(points: &[(&str, i64)]) -> Vec<(String, i64)> {
    points
        .iter()
        .map(|(date, cents)| (date.to_string(), *cents))
        .collect()
}

async fn history(browser: &Session, query: &str) -> Value {
    let response = browser.get(&format!("/balance/history?{query}")).await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

#[tokio::test]
async fn the_daily_history_is_the_current_balance_at_the_end_of_each_day() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    record(&ana, "expense", 10_000, "2026-01-05").await; // before the period
    record(&ana, "income", 50_000, "2026-01-10").await;
    record(&ana, "expense", 20_000, "2026-01-12").await;

    let body = history(&ana, "from=2026-01-09&to=2026-01-13&granularity=day").await;

    assert_eq!(
        points(&body),
        expected(&[
            ("2026-01-09", 90_000),
            ("2026-01-10", 140_000),
            ("2026-01-11", 140_000),
            ("2026-01-12", 120_000),
            ("2026-01-13", 120_000),
        ])
    );
}

#[tokio::test]
async fn the_monthly_history_has_one_point_per_month_cut_at_the_end_of_the_period() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    record(&ana, "income", 50_000, "2026-01-20").await;
    record(&ana, "expense", 30_000, "2026-02-28").await;
    record(&ana, "expense", 10_000, "2026-03-05").await;
    record(&ana, "expense", 5_000, "2026-03-20").await; // a Projection
    app.set_today(2026, 3, 10);

    let body = history(&ana, "from=2026-01-10&to=2026-03-10&granularity=month").await;

    assert_eq!(
        points(&body),
        expected(&[
            ("2026-01-31", 150_000),
            ("2026-02-28", 120_000),
            ("2026-03-10", 110_000), // the last month stops at `to`
        ])
    );
    assert_eq!(body["granularity"], json!("month"));
}

#[tokio::test]
async fn the_history_counts_the_occurrences_that_had_already_fallen() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 1_000_000).await;
    create_recurring(&ana, "expense", 150_000, 5).await;
    app.set_today(2026, 3, 20);

    let body = history(&ana, "from=2026-01-01&to=2026-03-20&granularity=month").await;

    assert_eq!(
        points(&body),
        expected(&[
            ("2026-01-31", 850_000),
            ("2026-02-28", 700_000),
            ("2026-03-20", 550_000),
        ])
    );
}

#[tokio::test]
async fn the_history_defaults_to_the_last_thirty_days_up_to_today() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    record(&ana, "income", 1_000, "2026-01-15").await;

    let body = history(&ana, "").await;

    let all = points(&body);
    assert_eq!(all.len(), 30);
    assert_eq!(all.first().unwrap(), &("2025-12-17".to_string(), 0));
    assert_eq!(all.last().unwrap(), &("2026-01-15".to_string(), 1_000));
    assert_eq!(body["granularity"], json!("day"));
    assert_eq!(body["from"], json!("2025-12-17"));
    assert_eq!(body["to"], json!("2026-01-15"));
}

#[tokio::test]
async fn the_history_refuses_a_period_that_goes_past_today_or_is_upside_down() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;

    let future = ana.get("/balance/history?to=2026-01-16").await;
    assert_eq!(future.status(), 422);
    let body: Value = future.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["to"], json!("history_in_future"));

    let upside_down = ana
        .get("/balance/history?from=2026-01-10&to=2026-01-05")
        .await;
    assert_eq!(upside_down.status(), 422);
    let body: Value = upside_down.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["to"], json!("period_order"));

    let too_long = ana
        .get("/balance/history?from=2020-01-01&to=2026-01-15&granularity=day")
        .await;
    assert_eq!(too_long.status(), 422);
    let body: Value = too_long.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["from"], json!("period_too_long"));

    let unknown = ana.get("/balance/history?granularity=year").await;
    assert_eq!(unknown.status(), 400);
    let body: Value = unknown.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("invalid_request"));
}

async fn projection(browser: &Session, query: &str) -> Value {
    let response = browser.get(&format!("/balance/projection?{query}")).await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// Ana has R$ 1.000,00 opening, a R$ 200,00 income on the 10th, a R$ 400,00
/// Projection on the 20th, a R$ 1.500,00 monthly expense on the 5th and a
/// R$ 3.000,00 monthly income on the 28th. On 2026-01-15 the Current Balance
/// is -R$ 300,00: the January rent has already fallen.
async fn ana_with_a_projection_and_two_fixed_transactions(app: &TestApp) -> Session {
    let ana = signed_up(app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    record(&ana, "income", 20_000, "2026-01-10").await;
    record(&ana, "expense", 40_000, "2026-01-20").await;
    create_recurring(&ana, "expense", 150_000, 5).await;
    create_recurring(&ana, "income", 300_000, 28).await;
    ana
}

#[tokio::test]
async fn the_projected_balance_adds_projections_and_occurrences_month_by_month() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = ana_with_a_projection_and_two_fixed_transactions(&app).await;

    let body = projection(&ana, "to=2026-04-30&granularity=month").await;

    assert_eq!(body["current_balance_cents"], json!(-30_000));
    assert_eq!(body["today"], json!("2026-01-15"));
    assert_eq!(body["to"], json!("2026-04-30"));
    assert_eq!(
        points(&body),
        expected(&[
            ("2026-01-31", 230_000), // -300,00 - 400,00 + 3.000,00
            ("2026-02-28", 380_000),
            ("2026-03-31", 530_000),
            ("2026-04-30", 680_000),
        ])
    );
}

#[tokio::test]
async fn the_daily_projection_starts_at_the_current_balance_and_moves_with_the_clock() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = ana_with_a_projection_and_two_fixed_transactions(&app).await;

    let body = projection(&ana, "to=2026-01-21&granularity=day").await;

    assert_eq!(
        points(&body),
        expected(&[
            ("2026-01-15", -30_000), // the Current Balance
            ("2026-01-16", -30_000),
            ("2026-01-17", -30_000),
            ("2026-01-18", -30_000),
            ("2026-01-19", -30_000),
            ("2026-01-20", -70_000), // the Projection falls
            ("2026-01-21", -70_000),
        ])
    );

    // On the 20th the Projection is no longer one: the Current Balance has
    // absorbed it and the projection starts from there.
    app.set_today(2026, 1, 20);
    let body = projection(&ana, "to=2026-01-21&granularity=day").await;
    assert_eq!(body["current_balance_cents"], json!(-70_000));
    assert_eq!(
        points(&body),
        expected(&[("2026-01-20", -70_000), ("2026-01-21", -70_000)])
    );
}

#[tokio::test]
async fn the_projection_reaches_twelve_months_ahead_and_no_further() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 10_000).await;

    let by_default = projection(&ana, "granularity=month").await;
    let all = points(&by_default);
    assert_eq!(all.len(), 13); // this month and the 12 after it
    assert_eq!(all.last().unwrap(), &("2027-01-31".to_string(), 10_000));
    assert_eq!(by_default["to"], json!("2027-01-31"));

    let last_day = projection(&ana, "to=2027-01-31&granularity=day").await;
    assert_eq!(points(&last_day).last().unwrap().0, "2027-01-31");

    let beyond = ana
        .get("/balance/projection?to=2027-02-01&granularity=day")
        .await;
    assert_eq!(beyond.status(), 422);
    let body: Value = beyond.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["to"], json!("beyond_horizon"));

    let past = ana.get("/balance/projection?to=2026-01-14").await;
    assert_eq!(past.status(), 422);
    let body: Value = past.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["to"], json!("projection_in_past"));
}

#[tokio::test]
async fn the_projection_leaves_out_an_occurrence_the_user_deleted() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    create_recurring(&ana, "expense", 100_000, 20).await;
    let list: Value = ana
        .get("/recurring-transactions")
        .await
        .json()
        .await
        .unwrap();
    let id = list["recurring_transactions"][0]["id"].as_str().unwrap();
    // The user deletes February's Occurrence only.
    let response = ana
        .delete(&format!("/recurring-transactions/{id}/occurrences/2026-02"))
        .await;
    assert_eq!(response.status(), 204);

    let body = projection(&ana, "to=2026-03-31&granularity=month").await;

    assert_eq!(
        points(&body),
        expected(&[
            ("2026-01-31", -100_000),
            ("2026-02-28", -100_000),
            ("2026-03-31", -200_000),
        ])
    );
}

#[tokio::test]
async fn each_user_sees_only_their_own_series_and_a_session_is_needed() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    set_opening(&ana, 70_000).await;

    let ana_history = history(&ana, "granularity=day").await;
    let bia_history = history(&bia, "granularity=day").await;
    assert_eq!(points(&ana_history).last().unwrap().1, 70_000);
    assert_eq!(points(&bia_history).last().unwrap().1, 0);
    let bia_projection = projection(&bia, "granularity=month").await;
    assert_eq!(points(&bia_projection).last().unwrap().1, 0);

    let visitor = app.session();
    assert_eq!(visitor.get("/balance/history").await.status(), 401);
    assert_eq!(visitor.get("/balance/projection").await.status(), 401);
}
