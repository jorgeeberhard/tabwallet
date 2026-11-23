//! Correcting the payment of an invoice (UC20) and the Carried Balance it
//! leaves on the next invoice, through the HTTP API (ADR-0002, ADR-0003). Each
//! test moves "today" with `set_today`.
//!
//! The cards used close on day 20 and are due on day 27, so the invoice that
//! closes in January is Open until 2026-01-20, Closed until 2026-01-26 and Paid
//! from 2026-01-27 (the Due Day); the February one is due on 2026-02-27, and so
//! on.

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

async fn invoice(browser: &Session, card_id: &str, closing_month: &str) -> Value {
    let response = browser
        .get(&format!("/credit-cards/{card_id}/invoices/{closing_month}"))
        .await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// Corrects the payment of the invoice to the amount actually paid.
async fn correct(
    browser: &Session,
    card_id: &str,
    closing_month: &str,
    amount_cents: i64,
) -> reqwest::Response {
    browser
        .put_json(
            &format!("/credit-cards/{card_id}/invoices/{closing_month}/payment"),
            &json!({ "amount_cents": amount_cents }),
        )
        .await
}

async fn error_code(response: reqwest::Response) -> (u16, Value) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    (status, body["error"].clone())
}

// --- Corrected payment and Carried Balance ------------------------------

#[tokio::test]
async fn paying_less_than_the_invoice_carries_the_difference_to_the_next_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    purchase(&ana, &nubank, 5_000, "2026-02-01").await;
    app.set_today(2026, 1, 27);
    // The Due Day debited the whole invoice.
    assert_eq!(balance(&ana).await, 70_000);

    let response = correct(&ana, &nubank, "2026-01", 20_000).await;

    let status = response.status();
    let body: Value = response.json().await.unwrap();
    assert_eq!(status, 200, "{body}");
    // The original invoice is Paid: nothing remains, the difference moved on.
    assert_eq!(body["invoice"]["state"], json!("paid"));
    assert_eq!(body["invoice"]["total_cents"], json!(30_000));
    assert_eq!(body["invoice"]["paid_cents"], json!(20_000));
    assert_eq!(body["invoice"]["remaining_cents"], json!(0));
    assert_eq!(body["invoice"]["carried_forward_cents"], json!(10_000));
    assert_eq!(body["payment"]["origin"], json!("corrected"));
    assert_eq!(body["payment"]["amount_cents"], json!(20_000));
    // Only what was really paid left the balance.
    assert_eq!(balance(&ana).await, 80_000);

    // The next invoice shows the Carried Balance as an item and owes it.
    let next = invoice(&ana, &nubank, "2026-02").await;
    assert_eq!(next["invoice"]["state"], json!("open"));
    assert_eq!(next["invoice"]["carried_balance_cents"], json!(10_000));
    assert_eq!(next["invoice"]["total_cents"], json!(15_000));
    assert_eq!(next["invoice"]["remaining_cents"], json!(15_000));
    assert_eq!(
        next["carried_balance"],
        json!({ "amount_cents": 10_000, "from_closing_month": "2026-01" })
    );
    // It is not an expense: the purchases of the invoice are the only items.
    assert_eq!(next["purchases"].as_array().unwrap().len(), 1);
    // The Carried Balance alone did not move the balance...
    assert_eq!(balance(&ana).await, 80_000);
    // ...it leaves with the next invoice, on its Due Day.
    app.set_today(2026, 2, 27);
    assert_eq!(balance(&ana).await, 65_000);
}

#[tokio::test]
async fn an_invoice_paid_in_full_has_no_carried_balance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);

    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(read["invoice"]["carried_forward_cents"], json!(0));
    let next = invoice(&ana, &nubank, "2026-02").await;
    assert_eq!(next["invoice"]["carried_balance_cents"], json!(0));
    assert_eq!(next["carried_balance"], json!(null));
}

#[tokio::test]
async fn correcting_again_recomputes_the_carried_balance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);
    assert_eq!(
        correct(&ana, &nubank, "2026-01", 20_000).await.status(),
        200
    );

    let response = correct(&ana, &nubank, "2026-01", 25_000).await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["invoice"]["carried_forward_cents"], json!(5_000));
    assert_eq!(body["invoice"]["paid_cents"], json!(25_000));
    assert_eq!(balance(&ana).await, 75_000);
    let next = invoice(&ana, &nubank, "2026-02").await;
    assert_eq!(next["invoice"]["carried_balance_cents"], json!(5_000));
    assert_eq!(next["invoice"]["total_cents"], json!(5_000));

    // Paying it all after all leaves nothing to carry.
    assert_eq!(
        correct(&ana, &nubank, "2026-01", 30_000).await.status(),
        200
    );
    assert_eq!(balance(&ana).await, 70_000);
    let next = invoice(&ana, &nubank, "2026-02").await;
    assert_eq!(next["invoice"]["carried_balance_cents"], json!(0));
    assert_eq!(next["carried_balance"], json!(null));
}

#[tokio::test]
async fn paying_nothing_carries_the_whole_unpaid_part() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);

    let response = correct(&ana, &nubank, "2026-01", 0).await;

    assert_eq!(response.status(), 200);
    assert_eq!(balance(&ana).await, 100_000);
    let next = invoice(&ana, &nubank, "2026-02").await;
    assert_eq!(next["invoice"]["carried_balance_cents"], json!(30_000));
    assert_eq!(next["invoice"]["remaining_cents"], json!(30_000));
}

#[tokio::test]
async fn the_correction_only_covers_what_the_early_payments_left_unpaid() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    let early = ana
        .post_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payments"),
            &json!({ "amount_cents": 10_000 }),
        )
        .await;
    assert_eq!(early.status(), 201);
    app.set_today(2026, 1, 27);
    // 10_000 was paid early; the due date debited the other 20_000.
    assert_eq!(balance(&ana).await, 70_000);

    let (status, error) = error_code(correct(&ana, &nubank, "2026-01", 20_001).await).await;
    assert_eq!(status, 422);
    assert_eq!(
        error["fields"]["amount_cents"],
        json!("amount_exceeds_remaining")
    );

    let response = correct(&ana, &nubank, "2026-01", 5_000).await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["invoice"]["paid_cents"], json!(15_000));
    assert_eq!(body["invoice"]["carried_forward_cents"], json!(15_000));
    assert_eq!(balance(&ana).await, 85_000);
}

#[tokio::test]
async fn a_negative_amount_is_refused() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);

    let (status, error) = error_code(correct(&ana, &nubank, "2026-01", -1).await).await;

    assert_eq!(status, 422);
    assert_eq!(error["fields"]["amount_cents"], json!("amount_range"));
}

#[tokio::test]
async fn the_payment_can_only_be_corrected_once_the_invoice_is_due() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 26);

    let (status, error) = error_code(correct(&ana, &nubank, "2026-01", 100).await).await;

    assert_eq!(status, 409);
    assert_eq!(error["code"], json!("invoice_payment_not_due"));
    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(
        read["invoice"]["payment_correction"]["state"],
        json!("not_due")
    );
}

#[tokio::test]
async fn an_invoice_paid_in_full_early_has_no_payment_to_correct() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    let early = ana
        .post_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payments"),
            &json!({ "amount_cents": 30_000 }),
        )
        .await;
    assert_eq!(early.status(), 201);
    app.set_today(2026, 1, 27);

    let (status, error) = error_code(correct(&ana, &nubank, "2026-01", 0).await).await;

    assert_eq!(status, 409);
    assert_eq!(error["code"], json!("invoice_nothing_to_pay"));
}

#[tokio::test]
async fn the_payment_is_locked_once_the_invoice_that_received_the_carried_balance_is_paid() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);
    assert_eq!(
        correct(&ana, &nubank, "2026-01", 20_000).await.status(),
        200
    );

    // Still editable while February is not paid (its Due Day is the 27th).
    app.set_today(2026, 2, 26);
    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(
        read["invoice"]["payment_correction"]["state"],
        json!("editable")
    );
    assert_eq!(
        read["invoice"]["payment_correction"]["corrected_cents"],
        json!(20_000)
    );
    assert_eq!(
        correct(&ana, &nubank, "2026-01", 22_000).await.status(),
        200
    );

    app.set_today(2026, 2, 27);
    let balance_when_locked = balance(&ana).await;
    let (status, error) = error_code(correct(&ana, &nubank, "2026-01", 30_000).await).await;
    assert_eq!(status, 409);
    assert_eq!(error["code"], json!("invoice_payment_locked"));
    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(
        read["invoice"]["payment_correction"]["state"],
        json!("locked")
    );
    assert_eq!(balance(&ana).await, balance_when_locked);
    // 100_000 - 22_000 (January) - 8_000 (the Carried Balance, due in February).
    assert_eq!(balance_when_locked, 70_000);
}

#[tokio::test]
async fn an_invoice_never_corrected_is_locked_once_the_next_one_is_paid() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 2, 27);

    let (status, error) = error_code(correct(&ana, &nubank, "2026-01", 10_000).await).await;

    assert_eq!(status, 409);
    assert_eq!(error["code"], json!("invoice_payment_locked"));
}

#[tokio::test]
async fn the_carried_balance_can_be_carried_again_by_the_next_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    purchase(&ana, &nubank, 5_000, "2026-02-01").await;
    app.set_today(2026, 1, 27);
    assert_eq!(
        correct(&ana, &nubank, "2026-01", 20_000).await.status(),
        200
    );
    app.set_today(2026, 2, 27);
    // February owed its own 5_000 plus the 10_000 carried.
    assert_eq!(balance(&ana).await, 65_000);

    let response = correct(&ana, &nubank, "2026-02", 0).await;

    assert_eq!(response.status(), 200);
    assert_eq!(balance(&ana).await, 80_000);
    let march = invoice(&ana, &nubank, "2026-03").await;
    assert_eq!(march["invoice"]["carried_balance_cents"], json!(15_000));
    assert_eq!(
        march["carried_balance"],
        json!({ "amount_cents": 15_000, "from_closing_month": "2026-02" })
    );
    // January is now locked: February, which received its balance, is paid.
    let (status, _) = error_code(correct(&ana, &nubank, "2026-01", 30_000).await).await;
    assert_eq!(status, 409);
}

#[tokio::test]
async fn only_the_owner_can_correct_a_payment() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);

    let (status, error) = error_code(correct(&bia, &nubank, "2026-01", 100).await).await;
    assert_eq!(
        (status, error["code"].clone()),
        (404, json!("credit_card_not_found"))
    );
    let (status, error) = error_code(correct(&ana, &nubank, "2025-06", 100).await).await;
    assert_eq!(
        (status, error["code"].clone()),
        (404, json!("invoice_not_found"))
    );
    assert_eq!(balance(&ana).await, -30_000);
}

#[tokio::test]
async fn a_card_with_a_carried_balance_cannot_be_deactivated() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);
    assert_eq!(
        correct(&ana, &nubank, "2026-01", 20_000).await.status(),
        200
    );

    let response = ana
        .post_empty(&format!("/credit-cards/{nubank}/deactivate"))
        .await;

    let (status, error) = error_code(response).await;
    assert_eq!(status, 409);
    assert_eq!(error["code"], json!("credit_card_has_remaining_invoice"));
}

// --- Statement and Projected Balance ------------------------------------

#[tokio::test]
async fn the_statement_shows_the_corrected_payment_and_no_carried_balance_item() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);
    assert_eq!(
        correct(&ana, &nubank, "2026-01", 20_000).await.status(),
        200
    );

    let response = ana.get("/transactions").await;
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().unwrap();

    // The purchase, the corrected payment of January and the automatic one
    // that February will debit (a Projection); nothing for the Carried Balance.
    assert_eq!(items.len(), 3);
    let payments: Vec<&Value> = items
        .iter()
        .filter(|item| item["kind"] == json!("invoice_payment"))
        .collect();
    assert_eq!(payments.len(), 2);
    let corrected = payments
        .iter()
        .find(|payment| payment["origin"] == json!("corrected"))
        .unwrap();
    assert_eq!(corrected["amount_cents"], json!(20_000));
    assert_eq!(corrected["date"], json!("2026-01-27"));
    assert_eq!(corrected["is_projection"], json!(false));
    let february = payments
        .iter()
        .find(|payment| payment["origin"] == json!("automatic"))
        .unwrap();
    assert_eq!(february["amount_cents"], json!(10_000));
    assert_eq!(february["date"], json!("2026-02-27"));
    assert_eq!(february["is_projection"], json!(true));
}

#[tokio::test]
async fn the_projected_balance_follows_the_corrected_payment_and_the_carried_balance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    purchase(&ana, &nubank, 5_000, "2026-02-01").await;
    app.set_today(2026, 1, 27);
    assert_eq!(
        correct(&ana, &nubank, "2026-01", 20_000).await.status(),
        200
    );

    let body: Value = ana
        .get("/balance/projection?to=2026-03-31&granularity=month")
        .await
        .json()
        .await
        .unwrap();

    let points = body["points"].as_array().unwrap();
    // End of January: only the 20_000 really paid has left.
    assert_eq!(points[0]["balance_cents"], json!(80_000));
    // End of February: its own 5_000 plus the 10_000 carried were debited.
    assert_eq!(points[1]["balance_cents"], json!(65_000));
    assert_eq!(points[2]["balance_cents"], json!(65_000));
    // The history agrees with the Current Balance on each day.
    let history: Value = ana
        .get("/balance/history?from=2026-01-26&to=2026-01-27")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(history["points"][0]["balance_cents"], json!(100_000));
    assert_eq!(history["points"][1]["balance_cents"], json!(80_000));
}
