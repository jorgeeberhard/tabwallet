//! Purchases with a credit card and the invoices they enter (UC15, UC16 and
//! the card part of UC06, UC07 and UC08), through the HTTP API.

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

/// Registers a card and returns its id.
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

async fn purchase_in(
    browser: &Session,
    card_id: &str,
    category_id: &str,
    amount_cents: i64,
    date: &str,
) -> reqwest::Response {
    browser
        .post_json(
            "/card-purchases",
            &json!({
                "credit_card_id": card_id,
                "amount_cents": amount_cents,
                "date": date,
                "description": "compra",
                "category_id": category_id,
            }),
        )
        .await
}

/// Records a purchase in the default "Outros" category and returns it.
async fn purchase(browser: &Session, card_id: &str, amount_cents: i64, date: &str) -> Value {
    let category = category_id(browser, "Outros", "expense").await;
    let response = purchase_in(browser, card_id, &category, amount_cents, date).await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    body["card_purchase"].clone()
}

async fn invoices(browser: &Session, card_id: &str) -> Vec<Value> {
    let response = browser
        .get(&format!("/credit-cards/{card_id}/invoices"))
        .await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    body["invoices"].as_array().unwrap().clone()
}

fn invoice_closing(invoices: &[Value], month: &str) -> Value {
    invoices
        .iter()
        .find(|invoice| invoice["closing_month"] == json!(month))
        .unwrap_or_else(|| panic!("no invoice closing in {month}: {invoices:?}"))
        .clone()
}

// --- the closing rule --------------------------------------------------

#[tokio::test]
async fn a_purchase_on_the_closing_day_enters_the_invoice_closing_that_month() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    let bought = purchase(&ana, &nubank, 12_000, "2026-01-10").await;

    assert_eq!(bought["invoice"]["closing_month"], json!("2026-01"));
    assert_eq!(bought["invoice"]["closing_date"], json!("2026-01-10"));
    assert_eq!(bought["invoice"]["due_date"], json!("2026-01-17"));
    // The Reference Month is the month of the invoice's due date.
    assert_eq!(bought["reference_month"], json!("2026-01"));
    assert_eq!(bought["credit_card"]["name"], json!("Nubank"));
}

#[tokio::test]
async fn a_purchase_after_the_closing_day_enters_the_next_months_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    let bought = purchase(&ana, &nubank, 12_000, "2026-01-11").await;

    assert_eq!(bought["invoice"]["closing_month"], json!("2026-02"));
    assert_eq!(bought["invoice"]["closing_date"], json!("2026-02-10"));
    assert_eq!(bought["invoice"]["due_date"], json!("2026-02-17"));
    assert_eq!(bought["reference_month"], json!("2026-02"));
}

#[tokio::test]
async fn a_due_day_before_the_closing_day_puts_the_due_date_in_the_next_month() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let itau = card(&ana, "Itaú", 25, 5).await;

    let bought = purchase(&ana, &itau, 5_000, "2026-01-20").await;

    assert_eq!(bought["invoice"]["closing_date"], json!("2026-01-25"));
    assert_eq!(bought["invoice"]["due_date"], json!("2026-02-05"));
    // It counts in February, the month the money leaves.
    assert_eq!(bought["reference_month"], json!("2026-02"));
}

#[tokio::test]
async fn in_a_shorter_month_the_last_day_applies_to_the_closing_and_due_days() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let inter = card(&ana, "Inter", 31, 10).await;

    let bought = purchase(&ana, &inter, 5_000, "2026-02-28").await;

    assert_eq!(bought["invoice"]["closing_month"], json!("2026-02"));
    assert_eq!(bought["invoice"]["closing_date"], json!("2026-02-28"));
    assert_eq!(bought["invoice"]["due_date"], json!("2026-03-10"));
}

#[tokio::test]
async fn changing_the_date_reallocates_the_purchase_to_the_right_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 12_000, "2026-01-05").await;
    let id = bought["id"].as_str().unwrap();
    assert_eq!(bought["invoice"]["closing_month"], json!("2026-01"));

    let response = ana
        .patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "date": "2026-01-20" }),
        )
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["card_purchase"]["invoice"]["closing_month"],
        json!("2026-02")
    );
    let all = invoices(&ana, &nubank).await;
    assert_eq!(
        invoice_closing(&all, "2026-02")["total_cents"],
        json!(12_000)
    );
    // January's invoice is the first open one, and is now empty.
    assert!(
        all.iter()
            .all(|invoice| invoice["closing_month"] != json!("2026-01")
                || invoice["total_cents"] == json!(0))
    );
}

#[tokio::test]
async fn a_purchase_can_change_amount_description_and_category() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 12_000, "2026-01-05").await;
    let id = bought["id"].as_str().unwrap();
    let food = category_id(&ana, "Alimentação", "expense").await;

    let response = ana
        .patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "amount_cents": 9_990, "description": "  mercado ", "category_id": food }),
        )
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["card_purchase"]["amount_cents"], json!(9_990));
    assert_eq!(body["card_purchase"]["description"], json!("mercado"));
    assert_eq!(
        body["card_purchase"]["category"]["name"],
        json!("Alimentação")
    );
}

// --- the Current Balance ----------------------------------------------

#[tokio::test]
async fn a_card_purchase_does_not_change_the_current_balance() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    purchase(&ana, &nubank, 50_000, "2026-01-05").await;
    purchase(&ana, &nubank, 30_000, "2026-01-15").await;

    let balance: Value = ana.get("/balance").await.json().await.unwrap();

    assert_eq!(balance["current_balance_cents"], json!(0));
}

// --- invoice states ----------------------------------------------------

#[tokio::test]
async fn an_invoice_is_open_until_its_closing_day_closed_until_due_and_paid_after() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    purchase(&ana, &nubank, 12_000, "2026-01-05").await;

    app.set_today(2026, 1, 10); // the closing day itself: still open
    let january = invoice_closing(&invoices(&ana, &nubank).await, "2026-01");
    assert_eq!(january["state"], json!("open"));
    assert_eq!(january["remaining_cents"], json!(12_000));

    app.set_today(2026, 1, 11);
    let january = invoice_closing(&invoices(&ana, &nubank).await, "2026-01");
    assert_eq!(january["state"], json!("closed"));
    assert_eq!(january["total_cents"], json!(12_000));
    assert_eq!(january["remaining_cents"], json!(12_000));

    app.set_today(2026, 1, 16); // the day before it is due
    let january = invoice_closing(&invoices(&ana, &nubank).await, "2026-01");
    assert_eq!(january["state"], json!("closed"));

    app.set_today(2026, 1, 17); // the due day
    let january = invoice_closing(&invoices(&ana, &nubank).await, "2026-01");
    assert_eq!(january["state"], json!("paid"));
    assert_eq!(january["total_cents"], json!(12_000));
    assert_eq!(january["remaining_cents"], json!(0));
}

#[tokio::test]
async fn the_invoices_list_has_the_open_invoice_even_when_it_has_no_purchases() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    let all = invoices(&ana, &nubank).await;

    assert_eq!(all.len(), 1);
    assert_eq!(all[0]["closing_month"], json!("2026-02"));
    assert_eq!(all[0]["state"], json!("open"));
    assert_eq!(all[0]["total_cents"], json!(0));
}

#[tokio::test]
async fn an_invoice_detail_lists_only_its_own_purchases() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    purchase(&ana, &nubank, 1_000, "2026-01-05").await;
    purchase(&ana, &nubank, 2_000, "2026-01-10").await;
    purchase(&ana, &nubank, 4_000, "2026-01-11").await;

    let response = ana
        .get(&format!("/credit-cards/{nubank}/invoices/2026-01"))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["invoice"]["total_cents"], json!(3_000));
    assert_eq!(body["purchases"].as_array().unwrap().len(), 2);
    let missing = ana
        .get(&format!("/credit-cards/{nubank}/invoices/2025-06"))
        .await;
    assert_eq!(missing.status(), 404);
    let malformed = ana
        .get(&format!("/credit-cards/{nubank}/invoices/janeiro"))
        .await;
    assert_eq!(malformed.status(), 404);
}

// --- paid invoices are history ----------------------------------------

#[tokio::test]
async fn purchases_of_a_paid_invoice_cannot_be_edited_or_deleted() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 12_000, "2026-01-05").await;
    let id = bought["id"].as_str().unwrap();
    app.set_today(2026, 1, 17); // the invoice is paid from its due day

    let edit = ana
        .patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "amount_cents": 1 }),
        )
        .await;
    assert_eq!(edit.status(), 409);
    let body: Value = edit.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("invoice_paid"));
    let delete = ana.delete(&format!("/card-purchases/{id}")).await;
    assert_eq!(delete.status(), 409);

    let january = invoice_closing(&invoices(&ana, &nubank).await, "2026-01");
    assert_eq!(january["total_cents"], json!(12_000));
}

#[tokio::test]
async fn a_purchase_cannot_be_moved_or_added_into_a_paid_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 12_000, "2026-01-20").await; // February's invoice
    let id = bought["id"].as_str().unwrap();
    app.set_today(2026, 1, 17); // January's invoice is paid

    let category = category_id(&ana, "Outros", "expense").await;
    let create = purchase_in(&ana, &nubank, &category, 100, "2026-01-05").await;
    assert_eq!(create.status(), 422);
    let body: Value = create.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["date"], json!("invoice_paid"));
    let moved = ana
        .patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "date": "2026-01-05" }),
        )
        .await;
    assert_eq!(moved.status(), 422);
}

#[tokio::test]
async fn a_purchase_of_an_open_or_closed_invoice_can_be_deleted() {
    let app = TestApp::spawn().await; // today is 2026-01-15: January is closed
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 12_000, "2026-01-05").await;
    let id = bought["id"].as_str().unwrap();

    let response = ana.delete(&format!("/card-purchases/{id}")).await;

    assert_eq!(response.status(), 204);
    let january = invoice_closing(&invoices(&ana, &nubank).await.clone(), "2026-02");
    assert_eq!(january["total_cents"], json!(0));
}

// --- validation and ownership -----------------------------------------

#[tokio::test]
async fn a_purchase_needs_an_active_expense_category_and_a_positive_amount() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let salary = category_id(&ana, "Salário", "income").await;

    let income = purchase_in(&ana, &nubank, &salary, 1_000, "2026-01-05").await;
    assert_eq!(income.status(), 422);
    let body: Value = income.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["category_id"],
        json!("category_type_mismatch")
    );

    let other = category_id(&ana, "Outros", "expense").await;
    let zero = purchase_in(&ana, &nubank, &other, 0, "2026-01-05").await;
    assert_eq!(zero.status(), 422);
    let body: Value = zero.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["amount_cents"],
        json!("amount_range")
    );

    ana.post_empty(&format!("/categories/{other}/deactivate"))
        .await;
    let inactive = purchase_in(&ana, &nubank, &other, 1_000, "2026-01-05").await;
    assert_eq!(inactive.status(), 422);
    let body: Value = inactive.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["category_id"],
        json!("category_inactive")
    );
}

#[tokio::test]
async fn a_purchase_needs_an_existing_active_card_of_the_user() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let anas = card(&ana, "Nubank", 10, 17).await;
    let category = category_id(&bia, "Outros", "expense").await;

    let foreign = purchase_in(&bia, &anas, &category, 1_000, "2026-01-05").await;
    assert_eq!(foreign.status(), 422);
    let body: Value = foreign.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["credit_card_id"],
        json!("credit_card_not_found")
    );

    let bias = card(&bia, "Nubank", 10, 17).await;
    bia.post_empty(&format!("/credit-cards/{bias}/deactivate"))
        .await;
    let inactive = purchase_in(&bia, &bias, &category, 1_000, "2026-01-05").await;
    assert_eq!(inactive.status(), 422);
    let body: Value = inactive.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["credit_card_id"],
        json!("credit_card_inactive")
    );
}

#[tokio::test]
async fn a_user_cannot_see_or_change_another_users_purchases_and_invoices() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 12_000, "2026-01-05").await;
    let id = bought["id"].as_str().unwrap();

    assert_eq!(
        bia.patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "amount_cents": 1 })
        )
        .await
        .status(),
        404
    );
    assert_eq!(
        bia.delete(&format!("/card-purchases/{id}")).await.status(),
        404
    );
    assert_eq!(
        bia.get(&format!("/credit-cards/{nubank}/invoices"))
            .await
            .status(),
        404
    );
    let statement: Value = bia.get("/transactions").await.json().await.unwrap();
    assert_eq!(statement["total"], json!(0));
}

// --- changing the days only affects open invoices ----------------------

#[tokio::test]
async fn new_days_apply_to_open_invoices_and_closed_ones_keep_theirs() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    purchase(&ana, &nubank, 12_000, "2026-01-05").await; // January: closed since the 10th
    purchase(&ana, &nubank, 3_000, "2026-01-12").await; // February: open

    let response = ana
        .patch_json(
            &format!("/credit-cards/{nubank}"),
            &json!({ "closing_day": 20, "due_day": 25 }),
        )
        .await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["credit_card"]["closing_day"], json!(20));
    assert_eq!(body["credit_card"]["due_day"], json!(25));

    let all = invoices(&ana, &nubank).await;
    let january = invoice_closing(&all, "2026-01");
    assert_eq!(january["closing_date"], json!("2026-01-10"));
    assert_eq!(january["due_date"], json!("2026-01-17"));
    assert_eq!(january["total_cents"], json!(12_000));
    let february = invoice_closing(&all, "2026-02");
    assert_eq!(february["closing_date"], json!("2026-02-20"));
    assert_eq!(february["due_date"], json!("2026-02-25"));
    // A purchase after the old closing day, before the new one, now fits February.
    let bought = purchase(&ana, &nubank, 1_000, "2026-02-18").await;
    assert_eq!(bought["invoice"]["closing_month"], json!("2026-02"));
}

#[tokio::test]
async fn a_new_closing_day_that_already_passed_starts_with_the_next_invoice() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let itau = card(&ana, "Itaú", 25, 5).await;
    let january = purchase(&ana, &itau, 8_000, "2026-01-20").await; // closes on the 25th
    assert_eq!(january["invoice"]["closing_date"], json!("2026-01-25"));

    ana.patch_json(
        &format!("/credit-cards/{itau}"),
        &json!({ "closing_day": 10 }),
    )
    .await;

    // January's invoice is still open, but a closing day of the 10th would
    // already have passed: it keeps the 25th and no purchase moves.
    let all = invoices(&ana, &itau).await;
    let jan = invoice_closing(&all, "2026-01");
    assert_eq!(jan["closing_date"], json!("2026-01-25"));
    assert_eq!(jan["total_cents"], json!(8_000));
    let next = purchase(&ana, &itau, 1_000, "2026-02-09").await;
    assert_eq!(next["invoice"]["closing_date"], json!("2026-02-10"));
    assert_eq!(next["invoice"]["closing_month"], json!("2026-02"));
}

// --- deactivating -----------------------------------------------------

#[tokio::test]
async fn a_card_with_an_invoice_to_pay_cannot_be_deactivated_until_it_is_paid() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    purchase(&ana, &nubank, 12_000, "2026-01-05").await; // closed, due on the 17th

    let blocked = ana
        .post_empty(&format!("/credit-cards/{nubank}/deactivate"))
        .await;
    assert_eq!(blocked.status(), 409);
    let body: Value = blocked.json().await.unwrap();
    assert_eq!(
        body["error"]["code"],
        json!("credit_card_has_remaining_invoice")
    );

    app.set_today(2026, 1, 17);
    let allowed = ana
        .post_empty(&format!("/credit-cards/{nubank}/deactivate"))
        .await;
    assert_eq!(allowed.status(), 200);
    // History stays: the purchase is still in the statement and in its invoice.
    let statement: Value = ana.get("/transactions").await.json().await.unwrap();
    assert_eq!(statement["total"], json!(1));
    assert_eq!(statement["items"][0]["credit_card"]["active"], json!(false));
}

// --- the statement ----------------------------------------------------

#[tokio::test]
async fn the_statement_shows_card_purchases_with_card_and_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    purchase(&ana, &nubank, 12_000, "2026-01-11").await;

    let statement: Value = ana.get("/transactions").await.json().await.unwrap();

    assert_eq!(statement["total"], json!(1));
    let item = &statement["items"][0];
    assert_eq!(item["kind"], json!("card_purchase"));
    assert_eq!(item["amount_cents"], json!(12_000));
    assert_eq!(item["credit_card"]["name"], json!("Nubank"));
    assert_eq!(item["invoice"]["closing_month"], json!("2026-02"));
    assert_eq!(item["invoice"]["state"], json!("open"));
    assert_eq!(item["reference_month"], json!("2026-02"));
}

#[tokio::test]
async fn the_statement_mixes_transactions_and_purchases_and_filters_by_card() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let itau = card(&ana, "Itaú", 25, 5).await;
    let other = category_id(&ana, "Outros", "expense").await;
    let response = ana
        .post_json(
            "/transactions",
            &json!({
                "type": "expense", "amount_cents": 700, "date": "2026-01-08",
                "description": "padaria", "category_id": other,
            }),
        )
        .await;
    assert_eq!(response.status(), 201);
    purchase(&ana, &nubank, 12_000, "2026-01-12").await;
    purchase(&ana, &itau, 3_000, "2026-01-05").await;

    let all: Value = ana.get("/transactions").await.json().await.unwrap();
    assert_eq!(all["total"], json!(3));
    let kinds: Vec<&Value> = all["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| &item["kind"])
        .collect();
    assert_eq!(
        kinds,
        vec![
            &json!("card_purchase"),
            &json!("transaction"),
            &json!("card_purchase")
        ]
    );

    let only_nubank: Value = ana
        .get(&format!("/transactions?credit_card_id={nubank}"))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(only_nubank["total"], json!(1));
    assert_eq!(
        only_nubank["items"][0]["credit_card"]["name"],
        json!("Nubank")
    );

    let incomes: Value = ana
        .get("/transactions?type=income")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(incomes["total"], json!(0));
    let paged: Value = ana
        .get("/transactions?limit=2&offset=2")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(paged["total"], json!(3));
    assert_eq!(paged["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn the_card_filter_leaves_out_occurrences_and_the_statement_mixes_all_kinds() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    purchase(&ana, &nubank, 12_000, "2026-01-12").await;
    let housing = category_id(&ana, "Moradia", "expense").await;
    let rent = ana
        .post_json(
            "/recurring-transactions",
            &json!({
                "type": "expense", "amount_cents": 90_000, "day_of_month": 5,
                "description": "aluguel", "category_id": housing,
            }),
        )
        .await;
    assert_eq!(rent.status(), 201);

    let everything: Value = ana
        .get("/transactions?to=2026-02-28")
        .await
        .json()
        .await
        .unwrap();
    let kinds: Vec<&str> = everything["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"occurrence") && kinds.contains(&"card_purchase"));

    let only_card: Value = ana
        .get(&format!(
            "/transactions?to=2026-02-28&credit_card_id={nubank}"
        ))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(only_card["total"], json!(1));
    assert_eq!(only_card["items"][0]["kind"], json!("card_purchase"));
}
