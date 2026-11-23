//! Reports (UC11), through the HTTP API: expenses by Category for a period and
//! income x expenses per month, both by Reference Month. Each test moves
//! "today" with `set_today`.
//!
//! Unless a test says otherwise "today" is 2026-01-15 and the cards close on
//! the 10th and are due on the 17th, so a purchase made from 2026-01-11 to
//! 2026-02-10 is in the invoice due 2026-02-17 (Reference Month 2026-02).

mod common;

use std::collections::BTreeMap;

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

async fn card(browser: &Session, name: &str, closing_day: u32, due_day: u32) -> String {
    let response = browser
        .post_json(
            "/credit-cards",
            &json!({ "name": name, "closing_day": closing_day, "due_day": due_day }),
        )
        .await;
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    body["credit_card"]["id"].as_str().unwrap().to_string()
}

/// Records a Transaction outside the card.
async fn record(browser: &Session, kind: &str, category: &str, amount_cents: i64, date: &str) {
    let response = browser
        .post_json(
            "/transactions",
            &json!({
                "type": kind,
                "amount_cents": amount_cents,
                "date": date,
                "description": "test",
                "category_id": category_id(browser, category, kind).await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

/// Creates a recurring Transaction that starts in the current month.
async fn recurring(
    browser: &Session,
    kind: &str,
    category: &str,
    amount_cents: i64,
    day_of_month: u32,
) {
    let response = browser
        .post_json(
            "/recurring-transactions",
            &json!({
                "type": kind,
                "amount_cents": amount_cents,
                "day_of_month": day_of_month,
                "description": "fixed",
                "category_id": category_id(browser, category, kind).await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

async fn card_purchase(
    browser: &Session,
    card_id: &str,
    category: &str,
    amount_cents: i64,
    date: &str,
) {
    let response = browser
        .post_json(
            "/card-purchases",
            &json!({
                "credit_card_id": card_id,
                "amount_cents": amount_cents,
                "date": date,
                "description": "compra",
                "category_id": category_id(browser, category, "expense").await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

/// Launches an installment purchase and returns the ids of its Installments,
/// in order.
async fn installment_purchase(
    browser: &Session,
    card_id: &str,
    category: &str,
    total_cents: i64,
    count: i64,
    date: &str,
) -> Vec<String> {
    let response = browser
        .post_json(
            "/installment-purchases",
            &json!({
                "credit_card_id": card_id,
                "total_cents": total_cents,
                "installment_count": count,
                "date": date,
                "description": "geladeira",
                "category_id": category_id(browser, category, "expense").await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    body["installment_purchase"]["installments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|installment| installment["id"].as_str().unwrap().to_string())
        .collect()
}

async fn anticipate(browser: &Session, card_id: &str, ids: &[String], discount_cents: i64) {
    let response = browser
        .post_json(
            &format!("/credit-cards/{card_id}/installment-anticipations"),
            &json!({
                "installment_ids": ids,
                "discount_cents": discount_cents,
                "confirm": true,
            }),
        )
        .await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
}

async fn charge(browser: &Session, card_id: &str, closing_month: &str, cents: i64, date: &str) {
    let response = browser
        .post_json(
            &format!("/credit-cards/{card_id}/invoices/{closing_month}/charges"),
            &json!({ "amount_cents": cents, "date": date }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

async fn pay_early(browser: &Session, card_id: &str, closing_month: &str, cents: i64) {
    let response = browser
        .post_json(
            &format!("/credit-cards/{card_id}/invoices/{closing_month}/payments"),
            &json!({ "amount_cents": cents }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

async fn report(browser: &Session, path: &str) -> Value {
    let response = browser.get(path).await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    response.json().await.unwrap()
}

/// `name -> (total_cents, projected_cents)` of an expenses-by-category report.
fn by_category(body: &Value) -> BTreeMap<String, (i64, i64)> {
    body["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|category| {
            (
                category["name"].as_str().unwrap().to_string(),
                (
                    category["total_cents"].as_i64().unwrap(),
                    category["projected_cents"].as_i64().unwrap(),
                ),
            )
        })
        .collect()
}

fn totals(entries: &[(&str, i64, i64)]) -> BTreeMap<String, (i64, i64)> {
    entries
        .iter()
        .map(|(name, total, projected)| (name.to_string(), (*total, *projected)))
        .collect()
}

/// The month rows of an income-vs-expenses report as
/// `month -> (income, expense, projected_income, projected_expense)`.
fn months(body: &Value) -> BTreeMap<String, (i64, i64, i64, i64)> {
    body["months"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["month"].as_str().unwrap().to_string(),
                (
                    row["income_cents"].as_i64().unwrap(),
                    row["expense_cents"].as_i64().unwrap(),
                    row["projected_income_cents"].as_i64().unwrap(),
                    row["projected_expense_cents"].as_i64().unwrap(),
                ),
            )
        })
        .collect()
}

// --- expenses by Category ----------------------------------------------------

#[tokio::test]
async fn expenses_by_category_default_to_the_current_month_and_leave_incomes_out() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    record(&ana, "expense", "Alimentação", 3_000, "2026-01-03").await;
    record(&ana, "expense", "Alimentação", 2_000, "2026-01-20").await; // a Projection
    record(&ana, "expense", "Lazer", 9_000, "2026-01-10").await;
    record(&ana, "expense", "Lazer", 7_000, "2025-12-31").await; // last month
    record(&ana, "expense", "Lazer", 7_000, "2026-02-01").await; // next month
    record(&ana, "income", "Salário", 400_000, "2026-01-05").await;

    let body = report(&ana, "/reports/expenses-by-category").await;

    assert_eq!(body["from"], json!("2026-01"));
    assert_eq!(body["to"], json!("2026-01"));
    assert_eq!(body["total_cents"], json!(14_000));
    assert_eq!(body["projected_cents"], json!(2_000));
    assert_eq!(
        by_category(&body),
        totals(&[("Alimentação", 5_000, 2_000), ("Lazer", 9_000, 0)])
    );
    // The biggest first.
    assert_eq!(body["categories"][0]["name"], json!("Lazer"));
    assert!(body["categories"][0]["category_id"].is_string());
    assert_eq!(body["categories"][0]["active"], json!(true));
}

#[tokio::test]
async fn card_purchases_installments_anticipation_charges_and_recurring_count_together() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    // Outside the card, by its own date.
    record(&ana, "expense", "Alimentação", 4_000, "2026-02-03").await;
    // A fixed expense: January's Occurrence already fell, February's is a Projection.
    recurring(&ana, "expense", "Moradia", 100_000, 5).await;
    // Card purchases: the 5th is in the invoice due 2026-01-17 (January), the
    // 12th in the one due 2026-02-17 (February).
    card_purchase(&ana, &nubank, "Alimentação", 7_000, "2026-01-05").await;
    card_purchase(&ana, &nubank, "Alimentação", 20_000, "2026-01-12").await;
    // 300,01 in three: 100,01 + 100,00 + 100,00 on the invoices of Feb, Mar, Apr.
    let installments = installment_purchase(&ana, &nubank, "Lazer", 30_001, 3, "2026-01-12").await;
    // The last Installment comes to the open (February) invoice with 10,00 off.
    anticipate(&ana, &nubank, &installments[2..], 1_000).await;
    // Interest the bank charged on the February invoice.
    charge(&ana, &nubank, "2026-02", 1_250, "2026-01-14").await;
    // An Early Payment is not an expense.
    pay_early(&ana, &nubank, "2026-02", 10_000).await;

    let january = report(
        &ana,
        "/reports/expenses-by-category?from=2026-01&to=2026-01",
    )
    .await;
    assert_eq!(
        by_category(&january),
        totals(&[("Alimentação", 7_000, 0), ("Moradia", 100_000, 0)])
    );

    let february = report(
        &ana,
        "/reports/expenses-by-category?from=2026-02&to=2026-02",
    )
    .await;
    assert_eq!(
        by_category(&february),
        totals(&[
            // 40,00 outside the card (a Projection) + 200,00 on the card.
            ("Alimentação", 24_000, 4_000),
            ("Moradia", 100_000, 100_000),
            // 100,01 + the anticipated 100,00 less its 10,00 Discount.
            ("Lazer", 19_001, 0),
            ("Juros e encargos", 1_250, 0),
        ])
    );
    assert_eq!(february["total_cents"], json!(144_251));
    assert_eq!(february["projected_cents"], json!(104_000));

    // March keeps the Installment that was not anticipated; April has none.
    let march = report(
        &ana,
        "/reports/expenses-by-category?from=2026-03&to=2026-03",
    )
    .await;
    assert_eq!(
        by_category(&march),
        totals(&[("Lazer", 10_000, 0), ("Moradia", 100_000, 100_000)])
    );
    let april = report(
        &ana,
        "/reports/expenses-by-category?from=2026-04&to=2026-04",
    )
    .await;
    assert_eq!(
        by_category(&april),
        totals(&[("Moradia", 100_000, 100_000)])
    );

    // A longer period adds the months up.
    let quarter = report(
        &ana,
        "/reports/expenses-by-category?from=2026-01&to=2026-04",
    )
    .await;
    assert_eq!(
        by_category(&quarter),
        totals(&[
            ("Alimentação", 31_000, 4_000),
            ("Moradia", 400_000, 300_000),
            ("Lazer", 29_001, 0),
            ("Juros e encargos", 1_250, 0),
        ])
    );
}

#[tokio::test]
async fn invoice_payments_and_the_carried_balance_are_not_expenses() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 20, 27).await;
    card_purchase(&ana, &nubank, "Outros", 30_000, "2026-01-10").await;
    card_purchase(&ana, &nubank, "Outros", 5_000, "2026-02-01").await;
    app.set_today(2026, 1, 27);
    // The user paid only 200,00 of the 300,00 invoice: 100,00 is carried on.
    let response = ana
        .put_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payment"),
            &json!({ "amount_cents": 20_000 }),
        )
        .await;
    assert_eq!(response.status(), 200);
    let next: Value = ana
        .get(&format!("/credit-cards/{nubank}/invoices/2026-02"))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(next["invoice"]["total_cents"], json!(15_000));

    let report = report(
        &ana,
        "/reports/expenses-by-category?from=2026-01&to=2026-02",
    )
    .await;

    // 300,00 + 50,00: neither the payments nor the 100,00 carried balance.
    assert_eq!(by_category(&report), totals(&[("Outros", 35_000, 5_000)]));
    assert_eq!(report["total_cents"], json!(35_000));
}

#[tokio::test]
async fn an_invalid_period_is_refused() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    for path in [
        "/reports/expenses-by-category",
        "/reports/income-vs-expenses",
    ] {
        let upside_down = ana.get(&format!("{path}?from=2026-03&to=2026-01")).await;
        assert_eq!(upside_down.status(), 422);
        let body: Value = upside_down.json().await.unwrap();
        assert_eq!(body["error"]["fields"]["to"], json!("period_order"));

        let not_a_month = ana.get(&format!("{path}?from=2026-13")).await;
        assert_eq!(not_a_month.status(), 422);
        let body: Value = not_a_month.json().await.unwrap();
        assert_eq!(body["error"]["fields"]["from"], json!("invalid_month"));

        let too_long = ana.get(&format!("{path}?from=2020-01&to=2026-01")).await;
        assert_eq!(too_long.status(), 422);
        let body: Value = too_long.json().await.unwrap();
        assert_eq!(body["error"]["fields"]["from"], json!("period_too_long"));

        // Two years, reaching into the future, are fine.
        let long = ana.get(&format!("{path}?from=2025-01&to=2027-01")).await;
        assert_eq!(long.status(), 200);
    }
}

#[tokio::test]
async fn reports_are_private_to_each_user_and_need_a_session() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    record(&ana, "expense", "Lazer", 9_000, "2026-01-10").await;

    let hers = report(&bia, "/reports/expenses-by-category").await;
    assert_eq!(hers["total_cents"], json!(0));
    assert_eq!(hers["categories"], json!([]));

    let anonymous = app.session();
    for path in [
        "/reports/expenses-by-category",
        "/reports/income-vs-expenses",
    ] {
        assert_eq!(anonymous.get(path).await.status(), 401);
    }
}

// --- income x expenses --------------------------------------------------------

#[tokio::test]
async fn income_and_expenses_are_totalled_per_month_with_everything_that_counts() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    recurring(&ana, "income", "Salário", 500_000, 5).await;
    recurring(&ana, "expense", "Moradia", 100_000, 5).await;
    record(&ana, "income", "Outros", 30_000, "2026-03-20").await;
    card_purchase(&ana, &nubank, "Alimentação", 7_000, "2026-01-05").await;
    card_purchase(&ana, &nubank, "Alimentação", 20_000, "2026-01-12").await;
    let installments = installment_purchase(&ana, &nubank, "Lazer", 30_001, 3, "2026-01-12").await;
    anticipate(&ana, &nubank, &installments[2..], 1_000).await;
    charge(&ana, &nubank, "2026-02", 1_250, "2026-01-14").await;
    pay_early(&ana, &nubank, "2026-02", 10_000).await;

    let body = report(&ana, "/reports/income-vs-expenses?from=2026-01&to=2026-05").await;

    assert_eq!(body["from"], json!("2026-01"));
    assert_eq!(body["to"], json!("2026-05"));
    assert_eq!(
        months(&body),
        BTreeMap::from([
            // Income: the salary already received. Expenses: the rent and the
            // purchase of the invoice due on the 17th.
            ("2026-01".to_string(), (500_000, 107_000, 0, 0)),
            // Rent, 200,00 of purchases, 100,01 + 90,00 of Installments and
            // 12,50 of Charges; the salary and the rent are Projections.
            ("2026-02".to_string(), (500_000, 140_251, 500_000, 100_000)),
            // The Installment of March is no Projection: its purchase was made.
            ("2026-03".to_string(), (530_000, 110_000, 530_000, 100_000)),
            ("2026-04".to_string(), (500_000, 100_000, 500_000, 100_000)),
            ("2026-05".to_string(), (500_000, 100_000, 500_000, 100_000)),
        ])
    );
    // In order, with the difference and the grand totals.
    let first = &body["months"][0];
    assert_eq!(first["month"], json!("2026-01"));
    assert_eq!(first["balance_cents"], json!(393_000));
    assert_eq!(body["income_cents"], json!(2_530_000));
    assert_eq!(body["expense_cents"], json!(557_251));
}

#[tokio::test]
async fn the_monthly_series_covers_empty_months_and_defaults_to_the_last_six_months() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    record(&ana, "expense", "Lazer", 9_000, "2025-09-10").await;

    let body = report(&ana, "/reports/income-vs-expenses").await;

    assert_eq!(body["from"], json!("2025-08"));
    assert_eq!(body["to"], json!("2026-01"));
    let rows = months(&body);
    assert_eq!(rows.len(), 6);
    assert_eq!(rows["2025-09"], (0, 9_000, 0, 0));
    assert_eq!(rows["2025-08"], (0, 0, 0, 0));
    assert_eq!(rows["2026-01"], (0, 0, 0, 0));
}
