//! Charges (Encargos, UC21): the interest, penalty and IOF the bank charged on
//! an invoice, recorded by the user as an expense of the "Juros e encargos"
//! category and linked to that invoice, through the HTTP API. Each test moves
//! "today" with `set_today`.
//!
//! The cards used close on day 20 and are due on day 27 unless a test says
//! otherwise, so the invoice that closes in January is Open until 2026-01-20,
//! Closed until 2026-01-26 and Paid from 2026-01-27.

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

async fn card_with_days(browser: &Session, name: &str, closing_day: u32, due_day: u32) -> String {
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

async fn card(browser: &Session, name: &str) -> String {
    card_with_days(browser, name, 20, 27).await
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

/// Records Charges on the invoice.
async fn charge(
    browser: &Session,
    card_id: &str,
    closing_month: &str,
    amount_cents: i64,
    date: &str,
) -> reqwest::Response {
    browser
        .post_json(
            &format!("/credit-cards/{card_id}/invoices/{closing_month}/charges"),
            &json!({ "amount_cents": amount_cents, "date": date }),
        )
        .await
}

async fn charge_ok(
    browser: &Session,
    card_id: &str,
    closing_month: &str,
    amount_cents: i64,
    date: &str,
) -> Value {
    let response = charge(browser, card_id, closing_month, amount_cents, date).await;
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    assert_eq!(status, 201, "{body}");
    body
}

async fn error_code(response: reqwest::Response) -> (u16, Value) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    (status, body["error"].clone())
}

async fn statement(browser: &Session, query: &str) -> Vec<Value> {
    let response = browser.get(&format!("/transactions?{query}")).await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    body["items"].as_array().unwrap().clone()
}

fn charges_of(items: &[Value]) -> Vec<&Value> {
    items
        .iter()
        .filter(|item| item["kind"] == json!("charge"))
        .collect()
}

// --- Recording Charges --------------------------------------------------

#[tokio::test]
async fn charges_join_the_invoice_total_and_leave_the_balance_with_the_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;

    let body = charge_ok(&ana, &nubank, "2026-01", 1_250, "2026-01-12").await;

    assert_eq!(body["charge"]["amount_cents"], json!(1_250));
    assert_eq!(body["charge"]["date"], json!("2026-01-12"));
    assert_eq!(body["charge"]["description"], json!("Juros e encargos"));
    assert_eq!(
        body["charge"]["category"]["name"],
        json!("Juros e encargos")
    );
    assert_eq!(body["charge"]["category"]["type"], json!("expense"));
    assert_eq!(body["charge"]["invoice"]["closing_month"], json!("2026-01"));
    assert_eq!(body["charge"]["credit_card"]["name"], json!("Nubank"));
    assert_eq!(body["charge"]["is_projection"], json!(false));
    assert_eq!(body["invoice"]["total_cents"], json!(31_250));
    assert_eq!(body["invoice"]["charges_cents"], json!(1_250));
    assert_eq!(body["invoice"]["remaining_cents"], json!(31_250));
    // Like a purchase, a Charge does not move the Current Balance by itself...
    assert_eq!(balance(&ana).await, 100_000);
    // ...it is paid through the invoice payment.
    app.set_today(2026, 1, 27);
    assert_eq!(balance(&ana).await, 68_750);
    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(read["invoice"]["state"], json!("paid"));
    assert_eq!(read["invoice"]["paid_cents"], json!(31_250));
    let listed = read["charges"].as_array().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], body["charge"]["id"]);
    // The Charge is not a purchase of the invoice.
    assert_eq!(read["purchases"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_description_can_be_given_and_is_trimmed() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;

    let response = ana
        .post_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/charges"),
            &json!({ "amount_cents": 500, "date": "2026-01-12", "description": "  IOF  " }),
        )
        .await;

    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["charge"]["description"], json!("IOF"));
}

#[tokio::test]
async fn charges_count_in_the_month_of_the_invoices_due_date() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    // Closes on the 20th and is due on the 5th: the January invoice is due in
    // February.
    let inter = card_with_days(&ana, "Inter", 20, 5).await;
    purchase(&ana, &inter, 30_000, "2026-01-10").await;

    let body = charge_ok(&ana, &inter, "2026-01", 700, "2026-01-25").await;

    assert_eq!(body["charge"]["invoice"]["due_date"], json!("2026-02-05"));
    assert_eq!(body["charge"]["reference_month"], json!("2026-02"));
}

#[tokio::test]
async fn a_charge_is_linked_to_the_invoice_not_to_its_own_date() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    purchase(&ana, &nubank, 10_000, "2026-02-10").await;

    // Dated in February but charged on the January invoice.
    charge_ok(&ana, &nubank, "2026-01", 900, "2026-02-03").await;

    let january = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(january["invoice"]["total_cents"], json!(30_900));
    let february = invoice(&ana, &nubank, "2026-02").await;
    assert_eq!(february["invoice"]["total_cents"], json!(10_000));
}

#[tokio::test]
async fn a_charge_must_be_valid() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;

    for amount in [0, -5] {
        let (status, error) =
            error_code(charge(&ana, &nubank, "2026-01", amount, "2026-01-12").await).await;
        assert_eq!(status, 422);
        assert_eq!(error["fields"]["amount_cents"], json!("amount_range"));
    }
    let too_long = ana
        .post_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/charges"),
            &json!({ "amount_cents": 100, "date": "2026-01-12", "description": "x".repeat(201) }),
        )
        .await;
    let (status, error) = error_code(too_long).await;
    assert_eq!(status, 422);
    assert_eq!(error["fields"]["description"], json!("description_length"));
    let bad_date = charge(&ana, &nubank, "2026-01", 100, "2026-02-30").await;
    assert_eq!(bad_date.status(), 400);
}

#[tokio::test]
async fn charges_need_an_existing_invoice_of_the_users_own_card() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;

    let (status, error) =
        error_code(charge(&ana, &nubank, "2025-06", 100, "2026-01-12").await).await;
    assert_eq!(
        (status, error["code"].clone()),
        (404, json!("invoice_not_found"))
    );
    let (status, error) =
        error_code(charge(&bia, &nubank, "2026-01", 100, "2026-01-12").await).await;
    assert_eq!(
        (status, error["code"].clone()),
        (404, json!("credit_card_not_found"))
    );
}

#[tokio::test]
async fn a_paid_invoice_takes_no_new_charge() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);

    let (status, error) =
        error_code(charge(&ana, &nubank, "2026-01", 100, "2026-01-27").await).await;

    assert_eq!(status, 409);
    assert_eq!(error["code"], json!("invoice_paid"));
}

// --- Editing and deleting -----------------------------------------------

#[tokio::test]
async fn a_charge_can_be_edited_and_deleted_until_the_invoice_is_paid() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    let body = charge_ok(&ana, &nubank, "2026-01", 1_000, "2026-01-12").await;
    let id = body["charge"]["id"].as_str().unwrap();

    let response = ana
        .patch_json(
            &format!("/charges/{id}"),
            &json!({ "amount_cents": 1_500, "description": "Juros de janeiro" }),
        )
        .await;

    assert_eq!(response.status(), 200);
    let edited: Value = response.json().await.unwrap();
    assert_eq!(edited["charge"]["amount_cents"], json!(1_500));
    assert_eq!(edited["charge"]["description"], json!("Juros de janeiro"));
    assert_eq!(edited["invoice"]["total_cents"], json!(31_500));

    let deleted = ana.delete(&format!("/charges/{id}")).await;
    assert_eq!(deleted.status(), 204);
    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(read["invoice"]["total_cents"], json!(30_000));
    assert_eq!(read["charges"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn a_charge_of_a_paid_invoice_is_locked() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    let body = charge_ok(&ana, &nubank, "2026-01", 1_000, "2026-01-12").await;
    let id = body["charge"]["id"].as_str().unwrap();
    app.set_today(2026, 1, 27);

    let (status, error) = error_code(
        ana.patch_json(&format!("/charges/{id}"), &json!({ "amount_cents": 5 }))
            .await,
    )
    .await;
    assert_eq!(
        (status, error["code"].clone()),
        (409, json!("invoice_paid"))
    );
    let (status, error) = error_code(ana.delete(&format!("/charges/{id}")).await).await;
    assert_eq!(
        (status, error["code"].clone()),
        (409, json!("invoice_paid"))
    );
    let read = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(read["invoice"]["total_cents"], json!(31_000));
}

#[tokio::test]
async fn only_the_owner_can_change_a_charge() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    let body = charge_ok(&ana, &nubank, "2026-01", 1_000, "2026-01-12").await;
    let id = body["charge"]["id"].as_str().unwrap();

    let (status, error) = error_code(
        bia.patch_json(&format!("/charges/{id}"), &json!({ "amount_cents": 5 }))
            .await,
    )
    .await;
    assert_eq!(
        (status, error["code"].clone()),
        (404, json!("charge_not_found"))
    );
    let (status, _) = error_code(bia.delete(&format!("/charges/{id}")).await).await;
    assert_eq!(status, 404);
    let (status, _) = error_code(bia.delete("/charges/not-an-id").await).await;
    assert_eq!(status, 404);
}

// --- Statement ----------------------------------------------------------

#[tokio::test]
async fn charges_are_expenses_in_the_statement_linked_to_their_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    charge_ok(&ana, &nubank, "2026-01", 1_000, "2026-01-12").await;

    let all = statement(&ana, "").await;
    let found = charges_of(&all);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["amount_cents"], json!(1_000));
    assert_eq!(found[0]["invoice"]["closing_month"], json!("2026-01"));
    assert_eq!(found[0]["credit_card"]["id"], json!(nubank));
    assert_eq!(found[0]["reference_month"], json!("2026-01"));

    // An expense of the "Juros e encargos" category, on its card.
    assert_eq!(charges_of(&statement(&ana, "type=expense").await).len(), 1);
    assert_eq!(charges_of(&statement(&ana, "type=income").await).len(), 0);
    let category = category_id(&ana, "Juros e encargos", "expense").await;
    assert_eq!(
        charges_of(&statement(&ana, &format!("category_id={category}")).await).len(),
        1
    );
    let other = category_id(&ana, "Outros", "expense").await;
    assert_eq!(
        charges_of(&statement(&ana, &format!("category_id={other}")).await).len(),
        0
    );
    assert_eq!(
        charges_of(&statement(&ana, &format!("credit_card_id={nubank}")).await).len(),
        1
    );
}

// --- Category "Juros e encargos" ----------------------------------------

#[tokio::test]
async fn the_charges_category_cannot_be_deactivated_while_charges_exist() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    let body = charge_ok(&ana, &nubank, "2026-01", 1_000, "2026-01-12").await;
    let id = body["charge"]["id"].as_str().unwrap();
    let category = category_id(&ana, "Juros e encargos", "expense").await;

    let (status, error) = error_code(
        ana.post_empty(&format!("/categories/{category}/deactivate"))
            .await,
    )
    .await;
    assert_eq!(
        (status, error["code"].clone()),
        (409, json!("category_has_charges"))
    );

    // With no Charge left it can be deactivated.
    assert_eq!(ana.delete(&format!("/charges/{id}")).await.status(), 204);
    let response = ana
        .post_empty(&format!("/categories/{category}/deactivate"))
        .await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn charges_cannot_be_recorded_while_their_category_is_deactivated() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    let category = category_id(&ana, "Juros e encargos", "expense").await;
    let response = ana
        .post_empty(&format!("/categories/{category}/deactivate"))
        .await;
    assert_eq!(response.status(), 200);

    let (status, error) =
        error_code(charge(&ana, &nubank, "2026-01", 100, "2026-01-12").await).await;
    assert_eq!(status, 422);
    assert_eq!(error["fields"]["category_id"], json!("category_inactive"));

    let response = ana
        .post_empty(&format!("/categories/{category}/reactivate"))
        .await;
    assert_eq!(response.status(), 200);
    assert_eq!(
        charge(&ana, &nubank, "2026-01", 100, "2026-01-12")
            .await
            .status(),
        201
    );
}

// --- With the Carried Balance and Installments --------------------------

#[tokio::test]
async fn interest_on_the_next_invoice_is_paid_with_the_carried_balance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    app.set_today(2026, 1, 27);
    let response = ana
        .put_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payment"),
            &json!({ "amount_cents": 20_000 }),
        )
        .await;
    assert_eq!(response.status(), 200);

    // The bank charged interest on the unpaid 10_000, on the next invoice.
    let body = charge_ok(&ana, &nubank, "2026-02", 650, "2026-01-28").await;

    assert_eq!(body["invoice"]["carried_balance_cents"], json!(10_000));
    assert_eq!(body["invoice"]["charges_cents"], json!(650));
    assert_eq!(body["invoice"]["total_cents"], json!(10_650));
    assert_eq!(balance(&ana).await, 80_000);
    app.set_today(2026, 2, 27);
    assert_eq!(balance(&ana).await, 69_350);
    // The Carried Balance is not an expense: only the Charge is one.
    let items = statement(&ana, "type=expense").await;
    assert_eq!(charges_of(&items).len(), 1);
}

#[tokio::test]
async fn the_projected_balance_counts_the_charges_of_an_invoice_to_be_paid() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    purchase(&ana, &nubank, 30_000, "2026-01-10").await;
    charge_ok(&ana, &nubank, "2026-01", 1_000, "2026-01-12").await;

    let body: Value = ana
        .get("/balance/projection?to=2026-02-28&granularity=month")
        .await
        .json()
        .await
        .unwrap();

    let points = body["points"].as_array().unwrap();
    assert_eq!(points[0]["balance_cents"], json!(69_000));
    assert_eq!(points[1]["balance_cents"], json!(69_000));
    assert_eq!(balance(&ana).await, 100_000);
}

#[tokio::test]
async fn charges_and_the_carried_balance_work_on_invoices_with_installments() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    let category = category_id(&ana, "Outros", "expense").await;
    let response = ana
        .post_json(
            "/installment-purchases",
            &json!({
                "credit_card_id": nubank,
                "total_cents": 30_000,
                "installment_count": 3,
                "date": "2026-01-10",
                "description": "geladeira",
                "category_id": category,
            }),
        )
        .await;
    assert_eq!(response.status(), 201);
    // January, February and March each hold an Installment of 10_000.
    charge_ok(&ana, &nubank, "2026-01", 500, "2026-01-12").await;
    app.set_today(2026, 1, 27);
    assert_eq!(balance(&ana).await, 89_500);

    let response = ana
        .put_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payment"),
            &json!({ "amount_cents": 4_500 }),
        )
        .await;
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["invoice"]["total_cents"], json!(10_500));
    assert_eq!(body["invoice"]["carried_forward_cents"], json!(6_000));
    assert_eq!(balance(&ana).await, 95_500);

    let february = invoice(&ana, &nubank, "2026-02").await;
    assert_eq!(february["invoice"]["total_cents"], json!(16_000));
    assert_eq!(february["invoice"]["carried_balance_cents"], json!(6_000));
    assert_eq!(february["installments"].as_array().unwrap().len(), 1);
    app.set_today(2026, 2, 27);
    assert_eq!(balance(&ana).await, 79_500);
}

#[tokio::test]
async fn charges_and_the_carried_balance_work_on_invoices_with_anticipated_installments() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    set_opening(&ana, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    let category = category_id(&ana, "Outros", "expense").await;
    let response = ana
        .post_json(
            "/installment-purchases",
            &json!({
                "credit_card_id": nubank,
                "total_cents": 30_000,
                "installment_count": 3,
                "date": "2026-01-10",
                "description": "geladeira",
                "category_id": category,
            }),
        )
        .await;
    assert_eq!(response.status(), 201);
    let bought: Value = response.json().await.unwrap();
    let third = bought["installment_purchase"]["installments"][2]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // The March Installment comes to the open January invoice, with a discount.
    let response = ana
        .post_json(
            &format!("/credit-cards/{nubank}/installment-anticipations"),
            &json!({ "installment_ids": [third], "discount_cents": 500, "confirm": true }),
        )
        .await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    let january = invoice(&ana, &nubank, "2026-01").await;
    assert_eq!(january["invoice"]["total_cents"], json!(19_500));

    let body = charge_ok(&ana, &nubank, "2026-01", 500, "2026-01-16").await;
    assert_eq!(body["invoice"]["total_cents"], json!(20_000));
    app.set_today(2026, 1, 27);
    assert_eq!(balance(&ana).await, 80_000);

    let response = ana
        .put_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payment"),
            &json!({ "amount_cents": 15_000 }),
        )
        .await;
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["invoice"]["carried_forward_cents"], json!(5_000));
    assert_eq!(balance(&ana).await, 85_000);
    let february = invoice(&ana, &nubank, "2026-02").await;
    assert_eq!(february["invoice"]["total_cents"], json!(15_000));
}
