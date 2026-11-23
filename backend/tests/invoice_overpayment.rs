//! An invoice never comes to less than what was already paid on it early: a
//! change that would lower its total below the Early Payments (editing or
//! deleting a purchase, an Installment, a Charge, moving purchases with new
//! card days, anticipating Installments, correcting the payment of the
//! previous invoice) is refused with `409 invoice_would_be_overpaid`, through
//! the HTTP API.
//!
//! The cards used close on day 20 and are due on day 27, so the invoice that
//! closes in January is Open until 2026-01-20 and Paid from 2026-01-27.

mod common;

use common::{Session, TestApp};
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";
const CODE: &str = "invoice_would_be_overpaid";

async fn signed_up(app: &TestApp, email: &str) -> Session {
    let browser = app.session();
    let response = browser.sign_up(email, PASSWORD).await;
    assert_eq!(response.status(), 201);
    browser
}

async fn category_id(browser: &Session) -> String {
    let body: Value = browser.get("/categories").await.json().await.unwrap();
    body["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|category| {
            category["name"] == json!("Outros") && category["type"] == json!("expense")
        })
        .expect("the default expense category")["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn card(browser: &Session) -> String {
    let response = browser
        .post_json(
            "/credit-cards",
            &json!({ "name": "Nubank", "closing_day": 20, "due_day": 27 }),
        )
        .await;
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    body["credit_card"]["id"].as_str().unwrap().to_string()
}

/// Records a purchase and returns its id.
async fn purchase(browser: &Session, card_id: &str, amount_cents: i64, date: &str) -> String {
    let response = browser
        .post_json(
            "/card-purchases",
            &json!({
                "credit_card_id": card_id,
                "amount_cents": amount_cents,
                "date": date,
                "description": "compra",
                "category_id": category_id(browser).await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    body["card_purchase"]["id"].as_str().unwrap().to_string()
}

/// Records an installment purchase of three Installments and returns its id.
async fn installments(browser: &Session, card_id: &str, total_cents: i64, date: &str) -> String {
    let response = browser
        .post_json(
            "/installment-purchases",
            &json!({
                "credit_card_id": card_id,
                "total_cents": total_cents,
                "installment_count": 3,
                "date": date,
                "description": "geladeira",
                "category_id": category_id(browser).await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    body["installment_purchase"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn pay(browser: &Session, card_id: &str, closing_month: &str, amount_cents: i64) {
    let response = browser
        .post_json(
            &format!("/credit-cards/{card_id}/invoices/{closing_month}/payments"),
            &json!({ "amount_cents": amount_cents }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

async fn invoice(browser: &Session, card_id: &str, closing_month: &str) -> Value {
    let response = browser
        .get(&format!("/credit-cards/{card_id}/invoices/{closing_month}"))
        .await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

async fn total_of(browser: &Session, card_id: &str, closing_month: &str) -> i64 {
    invoice(browser, card_id, closing_month).await["invoice"]["total_cents"]
        .as_i64()
        .unwrap()
}

/// Asserts the refusal: `409 invoice_would_be_overpaid`.
async fn assert_overpaid(response: reqwest::Response) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"]["code"], json!(CODE), "{body}");
}

// --- card purchases ----------------------------------------------------

#[tokio::test]
async fn deleting_a_purchase_is_refused_when_the_invoice_would_come_to_less_than_was_paid() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 1, 10);
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana).await;
    let big = purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    let small = purchase(&ana, &nubank, 10_000, "2026-01-11").await;
    pay(&ana, &nubank, "2026-01", 30_000).await;

    // 30.000 is still covered without the small purchase.
    let response = ana.delete(&format!("/card-purchases/{small}")).await;
    assert_eq!(response.status(), 204);
    // Without the big one the invoice would come to 0 with 30.000 paid.
    assert_overpaid(ana.delete(&format!("/card-purchases/{big}")).await).await;

    assert_eq!(total_of(&ana, &nubank, "2026-01").await, 30_000);
}

#[tokio::test]
async fn editing_a_purchase_is_refused_when_it_would_leave_the_invoice_overpaid() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 1, 10);
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana).await;
    let id = purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    pay(&ana, &nubank, "2026-01", 30_000).await;

    let lower = ana
        .patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "amount_cents": 20_000 }),
        )
        .await;
    assert_overpaid(lower).await;
    // Moved to the invoice of February by its date, the one of January empties.
    let moved = ana
        .patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "date": "2026-01-25" }),
        )
        .await;
    assert_overpaid(moved).await;
    assert_eq!(total_of(&ana, &nubank, "2026-01").await, 30_000);

    // Raising it, or changing what does not touch the amount, is fine.
    let higher = ana
        .patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "amount_cents": 40_000 }),
        )
        .await;
    assert_eq!(higher.status(), 200);
}

// --- installment purchases ---------------------------------------------

#[tokio::test]
async fn editing_or_deleting_an_installment_purchase_is_refused_when_it_would_overpay() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 1, 10);
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana).await;
    // Three Installments of 10.000, in January, February and March.
    let id = installments(&ana, &nubank, 30_000, "2026-01-10").await;
    pay(&ana, &nubank, "2026-01", 10_000).await;

    let lower = ana
        .patch_json(
            &format!("/installment-purchases/{id}"),
            &json!({ "total_cents": 15_000 }),
        )
        .await;
    assert_overpaid(lower).await;
    assert_overpaid(ana.delete(&format!("/installment-purchases/{id}")).await).await;
    assert_eq!(total_of(&ana, &nubank, "2026-01").await, 10_000);

    let higher = ana
        .patch_json(
            &format!("/installment-purchases/{id}"),
            &json!({ "total_cents": 60_000 }),
        )
        .await;
    assert_eq!(higher.status(), 200);
}

#[tokio::test]
async fn cancelling_the_remaining_installments_is_refused_when_it_would_overpay() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 1, 10);
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana).await;
    let id = installments(&ana, &nubank, 30_000, "2026-01-10").await;
    // The invoice of January is paid; February's is open and paid early.
    app.set_today(2026, 1, 28);
    pay(&ana, &nubank, "2026-02", 10_000).await;

    assert_overpaid(ana.delete(&format!("/installment-purchases/{id}")).await).await;
    assert_eq!(total_of(&ana, &nubank, "2026-02").await, 10_000);
}

// --- charges -----------------------------------------------------------

#[tokio::test]
async fn editing_or_deleting_a_charge_is_refused_when_it_would_overpay() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 1, 10);
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana).await;
    purchase(&ana, &nubank, 10_000, "2026-01-10").await;
    let response = ana
        .post_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/charges"),
            &json!({ "amount_cents": 5_000, "date": "2026-01-10" }),
        )
        .await;
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    let charge = body["charge"]["id"].as_str().unwrap().to_string();
    pay(&ana, &nubank, "2026-01", 15_000).await;

    let lower = ana
        .patch_json(
            &format!("/charges/{charge}"),
            &json!({ "amount_cents": 2_000 }),
        )
        .await;
    assert_overpaid(lower).await;
    assert_overpaid(ana.delete(&format!("/charges/{charge}")).await).await;
    assert_eq!(total_of(&ana, &nubank, "2026-01").await, 15_000);

    let higher = ana
        .patch_json(
            &format!("/charges/{charge}"),
            &json!({ "amount_cents": 8_000 }),
        )
        .await;
    assert_eq!(higher.status(), 200);
}

// --- card days ---------------------------------------------------------

#[tokio::test]
async fn new_card_days_are_refused_when_they_move_a_purchase_out_of_a_paid_invoice() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 1, 10);
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana).await;
    // A purchase dated ahead, still in January's invoice (closing day 20).
    purchase(&ana, &nubank, 20_000, "2026-01-18").await;
    pay(&ana, &nubank, "2026-01", 20_000).await;

    // With closing day 15 it would go to February's invoice.
    let response = ana
        .patch_json(
            &format!("/credit-cards/{nubank}"),
            &json!({ "closing_day": 15 }),
        )
        .await;
    assert_overpaid(response).await;

    let cards: Value = ana.get("/credit-cards").await.json().await.unwrap();
    assert_eq!(cards["credit_cards"][0]["closing_day"], json!(20));
    assert_eq!(total_of(&ana, &nubank, "2026-01").await, 20_000);
}

// --- anticipation ------------------------------------------------------

#[tokio::test]
async fn anticipating_an_installment_is_refused_when_its_invoice_would_be_overpaid() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 1, 10);
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana).await;
    installments(&ana, &nubank, 30_000, "2026-01-10").await;
    pay(&ana, &nubank, "2026-03", 10_000).await;

    let body: Value = ana
        .get(&format!("/credit-cards/{nubank}/anticipable-installments"))
        .await
        .json()
        .await
        .unwrap();
    let march = body["installments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["invoice"]["closing_month"] == json!("2026-03"))
        .expect("the installment of March")["id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = ana
        .post_json(
            &format!("/credit-cards/{nubank}/installment-anticipations"),
            &json!({ "installment_ids": [march], "confirm": true }),
        )
        .await;
    assert_overpaid(response).await;
    assert_eq!(total_of(&ana, &nubank, "2026-03").await, 10_000);
}

// --- correcting the previous invoice's payment -------------------------

#[tokio::test]
async fn correcting_a_payment_is_refused_when_it_would_overpay_the_next_invoice() {
    let app = TestApp::spawn().await;
    app.set_today(2026, 1, 10);
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana).await;
    purchase(&ana, &nubank, 10_000, "2026-01-10").await;
    purchase(&ana, &nubank, 20_000, "2026-01-25").await;
    // January is paid (10.000 at the due date); she says she paid 4.000, so
    // 6.000 go to February as the Carried Balance.
    app.set_today(2026, 1, 28);
    let corrected = ana
        .put_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payment"),
            &json!({ "amount_cents": 4_000 }),
        )
        .await;
    assert_eq!(corrected.status(), 200);
    assert_eq!(total_of(&ana, &nubank, "2026-02").await, 26_000);
    pay(&ana, &nubank, "2026-02", 26_000).await;

    // Paying the whole of January would take the Carried Balance away.
    let again = ana
        .put_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payment"),
            &json!({ "amount_cents": 10_000 }),
        )
        .await;
    assert_overpaid(again).await;
    assert_eq!(total_of(&ana, &nubank, "2026-02").await, 26_000);
}
