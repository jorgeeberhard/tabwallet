//! Paying credit-card invoices (UC17, ADR-0003): Early Payments by the user and
//! the automatic payment of the Due Day, calculated on read (ADR-0002), through
//! the HTTP API. Each test moves "today" with `set_today`.
//!
//! The cards used close on day 20 and are due on day 27, so the invoice that
//! closes in January is Open until 2026-01-20, Closed until 2026-01-26 and Paid
//! from 2026-01-27 (the Due Day).

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

async fn card(browser: &Session, name: &str) -> String {
    let response = browser
        .post_json(
            "/credit-cards",
            &json!({ "name": name, "closing_day": 20, "due_day": 27 }),
        )
        .await;
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    body["credit_card"]["id"].as_str().unwrap().to_string()
}

/// A purchase of `amount_cents` in "Outros", on `date`.
async fn purchase(browser: &Session, card_id: &str, amount_cents: i64, date: &str) {
    let response = browser
        .post_json(
            "/card-purchases",
            &json!({
                "credit_card_id": card_id,
                "amount_cents": amount_cents,
                "date": date,
                "description": "compra",
                "category_id": category_id(browser, "Outros", "expense").await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

async fn set_opening(browser: &Session, amount_cents: i64) {
    let response = browser
        .post_json("/balance/opening", &json!({ "amount_cents": amount_cents }))
        .await;
    assert_eq!(response.status(), 201);
}

async fn balance(browser: &Session) -> i64 {
    let body: Value = browser.get("/balance").await.json().await.unwrap();
    body["current_balance_cents"].as_i64().unwrap()
}

async fn pay(
    browser: &Session,
    card_id: &str,
    closing_month: &str,
    amount_cents: i64,
) -> reqwest::Response {
    browser
        .post_json(
            &format!("/credit-cards/{card_id}/invoices/{closing_month}/payments"),
            &json!({ "amount_cents": amount_cents }),
        )
        .await
}

async fn invoice(browser: &Session, card_id: &str, closing_month: &str) -> Value {
    let response = browser
        .get(&format!("/credit-cards/{card_id}/invoices/{closing_month}"))
        .await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

async fn error_code(response: reqwest::Response) -> (u16, Value) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    (status, body["error"].clone())
}

/// The whole statement, newest first.
async fn statement(browser: &Session, query: &str) -> Vec<Value> {
    let response = browser.get(&format!("/transactions?{query}")).await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    body["items"].as_array().unwrap().clone()
}

fn payments_of(items: &[Value]) -> Vec<&Value> {
    items
        .iter()
        .filter(|item| item["kind"] == json!("invoice_payment"))
        .collect()
}

async fn projected_at(browser: &Session, to: &str) -> i64 {
    let body: Value = browser
        .get(&format!("/balance/projection?to={to}"))
        .await
        .json()
        .await
        .unwrap();
    body["points"].as_array().unwrap().last().unwrap()["balance_cents"]
        .as_i64()
        .unwrap()
}

// --- Early Payment -----------------------------------------------------

#[tokio::test]
async fn a_partial_early_payment_leaves_the_balance_and_lowers_the_remaining_amount() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    // The purchase alone does not touch the balance (ADR-0003).
    assert_eq!(balance(&ana).await, 100_000);

    let response = pay(&ana, &nubank, "2026-01", 12_000).await;

    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["payment"]["origin"], json!("early"));
    assert_eq!(body["payment"]["amount_cents"], json!(12_000));
    assert_eq!(body["payment"]["date"], json!("2026-01-15"));
    assert_eq!(body["invoice"]["state"], json!("open"));
    assert_eq!(body["invoice"]["total_cents"], json!(30_000));
    assert_eq!(body["invoice"]["paid_cents"], json!(12_000));
    assert_eq!(body["invoice"]["remaining_cents"], json!(18_000));
    assert_eq!(balance(&ana).await, 88_000);
    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(read["invoice"]["remaining_cents"], json!(18_000));
    assert_eq!(read["payments"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn a_closed_invoice_can_be_paid_in_full_early_but_stays_closed_until_the_due_day() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 22);

    let response = pay(&ana, &nubank, "2026-01", 30_000).await;

    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["invoice"]["state"], json!("closed"));
    assert_eq!(body["invoice"]["remaining_cents"], json!(0));
    assert_eq!(balance(&ana).await, -30_000);
    // On the Due Day nothing more is debited: the early payment covered it.
    app.set_today(2026, 1, 27);
    assert_eq!(balance(&ana).await, -30_000);
    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(read["invoice"]["state"], json!("paid"));
    assert_eq!(read["invoice"]["paid_cents"], json!(30_000));
    assert_eq!(read["payments"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn an_early_payment_cannot_be_more_than_the_remaining_amount() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    assert_eq!(pay(&ana, &nubank, "2026-01", 20_000).await.status(), 201);

    let (status, error) = error_code(pay(&ana, &nubank, "2026-01", 10_001).await).await;

    assert_eq!(status, 422);
    assert_eq!(error["code"], json!("validation_failed"));
    assert_eq!(
        error["fields"]["amount_cents"],
        json!("amount_exceeds_remaining")
    );
    // The exact remaining amount is fine; nothing else was debited before.
    assert_eq!(pay(&ana, &nubank, "2026-01", 10_000).await.status(), 201);
    assert_eq!(balance(&ana).await, 70_000);
}

#[tokio::test]
async fn an_early_payment_must_be_greater_than_zero() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;

    for amount in [0, -5] {
        let (status, error) = error_code(pay(&ana, &nubank, "2026-01", amount).await).await;
        assert_eq!(status, 422);
        assert_eq!(error["fields"]["amount_cents"], json!("amount_range"));
    }
}

#[tokio::test]
async fn a_paid_invoice_takes_no_early_payment() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);

    let (status, error) = error_code(pay(&ana, &nubank, "2026-01", 100).await).await;

    assert_eq!(status, 409);
    assert_eq!(error["code"], json!("invoice_paid"));
}

#[tokio::test]
async fn a_payment_needs_an_existing_invoice_of_the_users_own_card() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;

    let (status, error) = error_code(pay(&ana, &nubank, "2025-06", 100).await).await;
    assert_eq!(
        (status, error["code"].clone()),
        (404, json!("invoice_not_found"))
    );
    let (status, error) = error_code(pay(&bia, &nubank, "2026-01", 100).await).await;
    assert_eq!(
        (status, error["code"].clone()),
        (404, json!("credit_card_not_found"))
    );
    let (status, _) = error_code(pay(&bia, "not-an-id", "2026-01", 100).await).await;
    assert_eq!(status, 404);
    assert_eq!(balance(&ana).await, 0);
}

#[tokio::test]
async fn purchases_after_an_early_payment_only_raise_the_remaining_amount() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    assert_eq!(pay(&ana, &nubank, "2026-01", 30_000).await.status(), 201);
    assert_eq!(
        invoice(&ana, &nubank, "2026-01").await["invoice"]["remaining_cents"],
        json!(0)
    );

    purchase(&ana, &nubank, 5_000, "2026-01-16").await;

    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(read["invoice"]["total_cents"], json!(35_000));
    assert_eq!(read["invoice"]["paid_cents"], json!(30_000));
    assert_eq!(read["invoice"]["remaining_cents"], json!(5_000));
    // The balance only moved with the payment.
    assert_eq!(balance(&ana).await, 70_000);
}

// --- the automatic payment of the Due Day ------------------------------

#[tokio::test]
async fn on_the_due_day_the_unpaid_amount_is_debited_and_the_invoice_becomes_paid() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    assert_eq!(pay(&ana, &nubank, "2026-01", 10_000).await.status(), 201);

    app.set_today(2026, 1, 26);
    assert_eq!(balance(&ana).await, 90_000);
    let before = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(before["invoice"]["state"], json!("closed"));
    assert_eq!(before["invoice"]["remaining_cents"], json!(20_000));

    app.set_today(2026, 1, 27);
    assert_eq!(balance(&ana).await, 70_000);
    let after = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(after["invoice"]["state"], json!("paid"));
    assert_eq!(after["invoice"]["remaining_cents"], json!(0));
    assert_eq!(after["invoice"]["paid_cents"], json!(30_000));
    let automatic = after["payments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|payment| payment["origin"] == json!("automatic"))
        .unwrap();
    assert_eq!(automatic["amount_cents"], json!(20_000));
    assert_eq!(automatic["date"], json!("2026-01-27"));
    assert_eq!(automatic["id"], json!(null));
}

#[tokio::test]
async fn an_unpaid_invoice_is_debited_in_full_on_the_due_day_without_any_stored_row() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 50_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    purchase(&ana, &nubank, 7_000, "2026-01-12").await;

    app.set_today(2026, 1, 26);
    assert_eq!(balance(&ana).await, 50_000);
    app.set_today(2026, 1, 27);
    assert_eq!(balance(&ana).await, 13_000);
    // Still the same later, and the balance can go negative.
    app.set_today(2026, 3, 1);
    assert_eq!(balance(&ana).await, 13_000);
}

// --- the statement -----------------------------------------------------

#[tokio::test]
async fn the_statement_shows_early_and_automatic_payments_marked_with_the_card() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    assert_eq!(pay(&ana, &nubank, "2026-01", 10_000).await.status(), 201);

    // Before the Due Day the automatic payment is a projection.
    let items = statement(&ana, "").await;
    let payments = payments_of(&items);
    assert_eq!(payments.len(), 2);
    let automatic = payments
        .iter()
        .find(|item| item["origin"] == json!("automatic"))
        .unwrap();
    assert_eq!(automatic["amount_cents"], json!(20_000));
    assert_eq!(automatic["date"], json!("2026-01-27"));
    assert_eq!(automatic["is_projection"], json!(true));
    assert_eq!(automatic["credit_card"]["name"], json!("Nubank"));
    assert_eq!(automatic["invoice"]["closing_month"], json!("2026-01"));
    let early = payments
        .iter()
        .find(|item| item["origin"] == json!("early"))
        .unwrap();
    assert_eq!(early["amount_cents"], json!(10_000));
    assert_eq!(early["date"], json!("2026-01-15"));
    assert_eq!(early["is_projection"], json!(false));
    assert!(early["id"].is_string());

    app.set_today(2026, 1, 28);
    let items = statement(&ana, "").await;
    let automatic = payments_of(&items)
        .into_iter()
        .find(|item| item["origin"] == json!("automatic"))
        .unwrap()
        .clone();
    assert_eq!(automatic["is_projection"], json!(false));
    // The statement adds up to the Current Balance: opening balance minus the
    // payments that have fallen (the card purchase is not in it).
    let paid: i64 = payments_of(&items)
        .iter()
        .map(|item| item["amount_cents"].as_i64().unwrap())
        .sum();
    assert_eq!(balance(&ana).await, 100_000 - paid);
}

#[tokio::test]
async fn the_card_filter_applies_to_payments_and_type_and_category_filters_leave_them_out() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    let inter = card(&ana, "Inter").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    purchase(&ana, &inter, 9_000, "2026-01-11").await;
    assert_eq!(pay(&ana, &nubank, "2026-01", 10_000).await.status(), 201);
    assert_eq!(pay(&ana, &inter, "2026-01", 9_000).await.status(), 201);

    let only_nubank = statement(&ana, &format!("credit_card_id={nubank}")).await;
    let payments = payments_of(&only_nubank);
    assert_eq!(payments.len(), 2); // early + automatic of Nubank
    assert!(
        payments
            .iter()
            .all(|item| item["credit_card"]["name"] == json!("Nubank"))
    );
    assert!(payments_of(&statement(&ana, "type=expense").await).is_empty());
    assert!(payments_of(&statement(&ana, "type=income").await).is_empty());
    let category = category_id(&ana, "Outros", "expense").await;
    assert!(payments_of(&statement(&ana, &format!("category_id={category}")).await).is_empty());
}

#[tokio::test]
async fn the_statement_period_filter_applies_to_payment_dates() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    assert_eq!(pay(&ana, &nubank, "2026-01", 10_000).await.status(), 201);

    let january_first_half = statement(&ana, "from=2026-01-01&to=2026-01-20").await;
    let payments = payments_of(&january_first_half);
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0]["origin"], json!("early"));
    let total: Value = ana
        .get("/transactions?from=2026-01-21&to=2026-01-31")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(total["total"], json!(1));
}

// --- the Projected Balance ---------------------------------------------

#[tokio::test]
async fn the_projected_balance_subtracts_what_each_invoice_still_owes_by_its_due_day() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await; // January invoice, due 01-27
    purchase(&ana, &nubank, 8_000, "2026-01-25").await; // February invoice, due 02-27
    // Today is 2026-01-15: an Early Payment of part of the January invoice.
    assert_eq!(pay(&ana, &nubank, "2026-01", 10_000).await.status(), 201);

    // Before any due date: only the Early Payment has left.
    assert_eq!(projected_at(&ana, "2026-01-26").await, 90_000);
    // After the January due date: its remaining 20.000 comes off as well.
    assert_eq!(projected_at(&ana, "2026-01-27").await, 70_000);
    // The February invoice still is not due.
    assert_eq!(projected_at(&ana, "2026-02-26").await, 70_000);
    assert_eq!(projected_at(&ana, "2026-02-27").await, 62_000);
    // Later than every invoice: everything is paid.
    assert_eq!(projected_at(&ana, "2026-12-31").await, 62_000);
}

#[tokio::test]
async fn the_monthly_projection_counts_the_invoices_due_in_each_month() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    purchase(&ana, &nubank, 8_000, "2026-01-25").await;

    let body: Value = ana
        .get("/balance/projection?granularity=month&to=2026-03-31")
        .await
        .json()
        .await
        .unwrap();

    let points: Vec<(String, i64)> = body["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|point| {
            (
                point["date"].as_str().unwrap().to_string(),
                point["balance_cents"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        points,
        vec![
            ("2026-01-31".to_string(), 70_000),
            ("2026-02-28".to_string(), 62_000),
            ("2026-03-31".to_string(), 62_000),
        ]
    );
    // The Current Balance is still the opening balance.
    assert_eq!(body["current_balance_cents"], json!(100_000));
}

// --- the history -------------------------------------------------------

#[tokio::test]
async fn the_history_drops_on_the_day_of_each_payment() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    assert_eq!(pay(&ana, &nubank, "2026-01", 10_000).await.status(), 201); // 01-15
    app.set_today(2026, 1, 28);

    let body: Value = ana
        .get("/balance/history?from=2026-01-14&to=2026-01-28")
        .await
        .json()
        .await
        .unwrap();

    let at = |date: &str| -> i64 {
        body["points"]
            .as_array()
            .unwrap()
            .iter()
            .find(|point| point["date"] == json!(date))
            .unwrap()["balance_cents"]
            .as_i64()
            .unwrap()
    };
    assert_eq!(at("2026-01-14"), 100_000);
    assert_eq!(at("2026-01-15"), 90_000);
    assert_eq!(at("2026-01-26"), 90_000);
    assert_eq!(at("2026-01-27"), 70_000);
    assert_eq!(at("2026-01-28"), 70_000);
}

// --- ownership ---------------------------------------------------------

#[tokio::test]
async fn one_users_payments_never_reach_another_users_balance_or_statement() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    set_opening(&ana, 100_000).await;
    set_opening(&bia, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    assert_eq!(pay(&ana, &nubank, "2026-01", 10_000).await.status(), 201);

    assert_eq!(balance(&ana).await, 90_000);
    assert_eq!(balance(&bia).await, 100_000);
    assert!(payments_of(&statement(&bia, "").await).is_empty());
}
