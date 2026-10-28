//! Recurring Transactions and their Occurrences (UC05, UC06 and UC07), through
//! the HTTP API. The Occurrences are never stored: each test moves "today"
//! with `set_today` and reads them again.

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

/// The id of the user's category of this type with this name.
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

/// Creates a recurring Transaction in a default category and returns its id.
async fn create_recurring(
    browser: &Session,
    kind: &str,
    amount_cents: i64,
    day_of_month: u32,
    description: &str,
) -> String {
    let name = if kind == "income" {
        "Salário"
    } else {
        "Moradia"
    };
    let response = create_with(
        browser,
        json!({
            "type": kind,
            "amount_cents": amount_cents,
            "day_of_month": day_of_month,
            "description": description,
            "category_id": category_id(browser, name, kind).await,
        }),
    )
    .await;
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    body["recurring_transaction"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn create_with(browser: &Session, body: Value) -> reqwest::Response {
    browser.post_json("/recurring-transactions", &body).await
}

async fn current_balance(browser: &Session) -> i64 {
    let body: Value = browser.get("/balance").await.json().await.unwrap();
    body["current_balance_cents"].as_i64().unwrap()
}

#[tokio::test]
async fn an_occurrence_enters_the_balance_on_its_day_and_falls_on_the_last_day_of_short_months() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    create_recurring(&ana, "expense", 150_000, 31, "Aluguel").await;

    // Nothing has been stored for January yet, and the 31st has not come.
    assert_eq!(current_balance(&ana).await, 0);

    app.set_today(2026, 1, 31);
    assert_eq!(current_balance(&ana).await, -150_000);

    // February has no day 31: the Occurrence falls on the 28th.
    app.set_today(2026, 2, 27);
    assert_eq!(current_balance(&ana).await, -150_000);
    app.set_today(2026, 2, 28);
    assert_eq!(current_balance(&ana).await, -300_000);

    // 30-day month: the 30th. Then 31-day month: the 31st.
    app.set_today(2026, 4, 29);
    assert_eq!(current_balance(&ana).await, -450_000); // Jan, Feb, Mar
    app.set_today(2026, 4, 30);
    assert_eq!(current_balance(&ana).await, -600_000);
}

async fn statement(browser: &Session, query: &str) -> Value {
    let response = browser.get(&format!("/transactions{query}")).await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// `(date, amount_cents, is_projection)` of every Occurrence in a statement,
/// in the order the statement lists them.
fn occurrences(statement: &Value) -> Vec<(String, i64, bool)> {
    statement["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["kind"] == json!("occurrence"))
        .map(|item| {
            (
                item["date"].as_str().unwrap().to_string(),
                item["amount_cents"].as_i64().unwrap(),
                item["is_projection"].as_bool().unwrap(),
            )
        })
        .collect()
}

#[tokio::test]
async fn the_statement_lists_occurrences_newest_first_with_future_ones_as_projections() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;
    create_recurring(&ana, "income", 300_000, 20, "Salário").await;

    let body = statement(&ana, "").await;

    // Jan 2026 to Jan 2027: the projection window is 12 months ahead.
    let found = occurrences(&body);
    assert_eq!(found.len(), 26);
    assert_eq!(body["total"], json!(26));
    assert_eq!(found[0], ("2027-01-20".to_string(), 300_000, true));
    assert_eq!(found[1], ("2027-01-05".to_string(), 100_000, true));
    // The oldest two: the 5th already fell (history), the 20th is a Projection.
    assert_eq!(found[24], ("2026-01-20".to_string(), 300_000, true));
    assert_eq!(found[25], ("2026-01-05".to_string(), 100_000, false));
    let first = &body["items"][0];
    assert_eq!(first["description"], json!("Salário"));
    assert_eq!(first["type"], json!("income"));
    assert_eq!(first["category"]["name"], json!("Salário"));
    assert_eq!(first["month"], json!("2027-01"));

    // As the clock moves, the Occurrences that fell leave the Projections.
    app.set_today(2026, 3, 10);
    let found = occurrences(&statement(&ana, "?from=2026-02-01&to=2026-04-30").await);
    assert_eq!(
        found,
        vec![
            ("2026-04-20".to_string(), 300_000, true),
            ("2026-04-05".to_string(), 100_000, true),
            ("2026-03-20".to_string(), 300_000, true),
            ("2026-03-05".to_string(), 100_000, false),
            ("2026-02-20".to_string(), 300_000, false),
            ("2026-02-05".to_string(), 100_000, false),
        ]
    );
}

#[tokio::test]
async fn occurrences_follow_the_statement_filters_and_page_with_the_transactions() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;
    create_recurring(&ana, "income", 300_000, 20, "Salário").await;
    let groceries = category_id(&ana, "Alimentação", "expense").await;
    let response = ana
        .post_json(
            "/transactions",
            &json!({
                "type": "expense", "amount_cents": 4_000, "date": "2026-02-10",
                "description": "Mercado", "category_id": groceries,
            }),
        )
        .await;
    assert_eq!(response.status(), 201);

    // Type filter.
    let incomes = statement(&ana, "?type=income&from=2026-02-01&to=2026-03-31").await;
    assert_eq!(occurrences(&incomes).len(), 2);
    assert_eq!(incomes["total"], json!(2));

    // Category filter: the rent is "Moradia"; "Alimentação" has the groceries.
    let housing = category_id(&ana, "Moradia", "expense").await;
    let by_category = statement(&ana, &format!("?category_id={housing}&to=2026-03-31")).await;
    assert_eq!(by_category["total"], json!(3)); // 5 Jan, 5 Feb, 5 Mar
    let by_category = statement(&ana, &format!("?category_id={groceries}&to=2026-03-31")).await;
    assert_eq!(by_category["total"], json!(1));
    assert!(occurrences(&by_category).is_empty());

    // Pages hold Transactions and Occurrences in one newest-first order.
    let window = "from=2026-02-01&to=2026-02-28";
    let all = statement(&ana, &format!("?{window}")).await;
    assert_eq!(all["total"], json!(3));
    let order: Vec<String> = all["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["date"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(order, vec!["2026-02-20", "2026-02-10", "2026-02-05"]);
    let second_page = statement(&ana, &format!("?{window}&limit=2&offset=2")).await;
    assert_eq!(second_page["total"], json!(3));
    assert_eq!(second_page["items"].as_array().unwrap().len(), 1);
    assert_eq!(second_page["items"][0]["date"], json!("2026-02-05"));
}

fn occurrence_path(recurring_id: &str, month: &str) -> String {
    format!("/recurring-transactions/{recurring_id}/occurrences/{month}")
}

/// The Occurrences of the statement as `(YYYY-MM, amount_cents, is_edited)`,
/// oldest first, between two days.
async fn months_between(browser: &Session, from: &str, to: &str) -> Vec<(String, i64, bool)> {
    let body = statement(browser, &format!("?from={from}&to={to}")).await;
    let mut found: Vec<(String, i64, bool)> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["kind"] == json!("occurrence"))
        .map(|item| {
            (
                item["month"].as_str().unwrap().to_string(),
                item["amount_cents"].as_i64().unwrap(),
                item["is_edited"].as_bool().unwrap(),
            )
        })
        .collect();
    found.reverse();
    found
}

#[tokio::test]
async fn deleting_or_editing_one_occurrence_leaves_the_other_months_alone() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let rent = create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;

    let edited = ana
        .patch_json(
            &occurrence_path(&rent, "2026-03"),
            &json!({ "amount_cents": 80_000 }),
        )
        .await;
    assert_eq!(edited.status(), 200);
    let body: Value = edited.json().await.unwrap();
    assert_eq!(body["occurrence"]["amount_cents"], json!(80_000));
    assert_eq!(body["occurrence"]["date"], json!("2026-03-05"));
    assert_eq!(body["occurrence"]["is_edited"], json!(true));
    assert_eq!(
        ana.delete(&occurrence_path(&rent, "2026-04"))
            .await
            .status(),
        204
    );

    assert_eq!(
        months_between(&ana, "2026-02-01", "2026-05-31").await,
        vec![
            ("2026-02".to_string(), 100_000, false),
            ("2026-03".to_string(), 80_000, true),
            ("2026-05".to_string(), 100_000, false),
        ]
    );
    // Jan, Feb, edited Mar, no Apr, May.
    app.set_today(2026, 5, 10);
    assert_eq!(current_balance(&ana).await, -380_000);

    // A deleted Occurrence, one before the start and a malformed month do not
    // exist.
    for month in ["2026-04", "2025-12", "2026-13", "abril"] {
        let response = ana.delete(&occurrence_path(&rent, month)).await;
        assert_eq!(response.status(), 404, "month {month}");
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], json!("occurrence_not_found"));
    }
    let response = ana
        .patch_json(
            &occurrence_path(&rent, "2026-04"),
            &json!({ "amount_cents": 5 }),
        )
        .await;
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn an_occurrence_edit_needs_a_valid_amount_and_an_owner() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let rent = create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;

    let zero = ana
        .patch_json(
            &occurrence_path(&rent, "2026-03"),
            &json!({ "amount_cents": 0 }),
        )
        .await;
    assert_eq!(zero.status(), 422);
    let body: Value = zero.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["amount_cents"],
        json!("amount_range")
    );

    let foreign = bia
        .patch_json(
            &occurrence_path(&rent, "2026-03"),
            &json!({ "amount_cents": 5 }),
        )
        .await;
    assert_eq!(foreign.status(), 404);
    let body: Value = foreign.json().await.unwrap();
    assert_eq!(
        body["error"]["code"],
        json!("recurring_transaction_not_found")
    );
    assert_eq!(
        bia.delete(&occurrence_path(&rent, "2026-03"))
            .await
            .status(),
        404
    );
    // Nothing happened to Ana's.
    assert_eq!(
        months_between(&ana, "2026-03-01", "2026-03-31").await.len(),
        1
    );
}
