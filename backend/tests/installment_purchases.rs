//! Installment purchases on a credit card and their Installments (UC18 and
//! the installment part of UC06, UC07 and UC08), through the HTTP API.

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

async fn installment_purchase_in(
    browser: &Session,
    card_id: &str,
    category_id: &str,
    total_cents: i64,
    installment_count: i64,
    date: &str,
) -> reqwest::Response {
    browser
        .post_json(
            "/installment-purchases",
            &json!({
                "credit_card_id": card_id,
                "total_cents": total_cents,
                "installment_count": installment_count,
                "date": date,
                "description": "geladeira",
                "category_id": category_id,
            }),
        )
        .await
}

/// Launches an installment purchase in the default "Outros" category and
/// returns it.
async fn installment_purchase(
    browser: &Session,
    card_id: &str,
    total_cents: i64,
    installment_count: i64,
    date: &str,
) -> Value {
    let category = category_id(browser, "Outros", "expense").await;
    let response = installment_purchase_in(
        browser,
        card_id,
        &category,
        total_cents,
        installment_count,
        date,
    )
    .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    body["installment_purchase"].clone()
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

/// The amounts of the Installments, in order.
fn amounts(purchase: &Value) -> Vec<i64> {
    purchase["installments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|installment| installment["amount_cents"].as_i64().unwrap())
        .collect()
}

/// The closing months of the invoices the Installments are in, in order.
fn invoice_months(purchase: &Value) -> Vec<String> {
    purchase["installments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|installment| {
            installment["invoice"]["closing_month"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

// --- splitting and allocating ------------------------------------------

#[tokio::test]
async fn the_total_is_split_in_exact_cents_with_the_leftover_on_the_first_installment() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    // R$ 100,00 in 3x = 33,34 + 33,33 + 33,33.
    let bought = installment_purchase(&ana, &nubank, 10_000, 3, "2026-01-05").await;

    assert_eq!(amounts(&bought), vec![3_334, 3_333, 3_333]);
    assert_eq!(bought["total_cents"], json!(10_000));
    assert_eq!(bought["installment_count"], json!(3));
    let numbers: Vec<i64> = bought["installments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|installment| installment["number"].as_i64().unwrap())
        .collect();
    assert_eq!(numbers, vec![1, 2, 3]);
}

#[tokio::test]
async fn an_even_installment_total_is_split_without_leftover() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    let bought = installment_purchase(&ana, &nubank, 12_000, 4, "2026-01-05").await;

    assert_eq!(amounts(&bought), vec![3_000, 3_000, 3_000, 3_000]);
}

#[tokio::test]
async fn each_installment_goes_to_a_consecutive_invoice_starting_with_the_cash_one() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    // On or before the Closing Day: the January invoice would take the
    // purchase paid in full, so the first Installment goes there.
    let early = installment_purchase(&ana, &nubank, 10_000, 3, "2026-01-10").await;
    assert_eq!(
        invoice_months(&early),
        vec!["2026-01", "2026-02", "2026-03"]
    );

    // After it: the first Installment goes to the invoice of the next month.
    let late = installment_purchase(&ana, &nubank, 10_000, 3, "2026-01-11").await;
    assert_eq!(invoice_months(&late), vec!["2026-02", "2026-03", "2026-04"]);
}

#[tokio::test]
async fn installments_cross_the_end_of_the_year() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    let bought = installment_purchase(&ana, &nubank, 30_000, 3, "2026-11-20").await;

    assert_eq!(
        invoice_months(&bought),
        vec!["2026-12", "2027-01", "2027-02"]
    );
}

#[tokio::test]
async fn each_installment_counts_in_the_reference_month_of_its_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    // Closes on the 25th and is due on the 5th of the next month.
    let itau = card(&ana, "Itaú", 25, 5).await;

    let bought = installment_purchase(&ana, &itau, 9_000, 3, "2026-01-20").await;

    let installments = bought["installments"].as_array().unwrap();
    assert_eq!(installments[0]["invoice"]["due_date"], json!("2026-02-05"));
    assert_eq!(installments[0]["reference_month"], json!("2026-02"));
    assert_eq!(installments[1]["reference_month"], json!("2026-03"));
    assert_eq!(installments[2]["reference_month"], json!("2026-04"));
}

#[tokio::test]
async fn installments_add_to_the_totals_of_their_invoices() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    installment_purchase(&ana, &nubank, 10_000, 3, "2026-01-05").await;

    let listed = invoices(&ana, &nubank).await;
    assert_eq!(
        invoice_closing(&listed, "2026-01")["total_cents"],
        json!(3_334)
    );
    assert_eq!(
        invoice_closing(&listed, "2026-02")["total_cents"],
        json!(3_333)
    );
    assert_eq!(
        invoice_closing(&listed, "2026-03")["total_cents"],
        json!(3_333)
    );
    assert_eq!(
        invoice_closing(&listed, "2026-03")["remaining_cents"],
        json!(3_333)
    );
}

#[tokio::test]
async fn installments_do_not_change_the_current_balance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let before: Value = ana.get("/balance").await.json().await.unwrap();

    installment_purchase(&ana, &nubank, 10_000, 3, "2026-01-05").await;

    let after: Value = ana.get("/balance").await.json().await.unwrap();
    assert_eq!(after, before);
}

// --- validation --------------------------------------------------------

#[tokio::test]
async fn the_number_of_installments_is_between_two_and_forty_eight() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let category = category_id(&ana, "Outros", "expense").await;

    for count in [0, 1, 49, -3] {
        let response =
            installment_purchase_in(&ana, &nubank, &category, 10_000, count, "2026-01-05").await;
        assert_eq!(response.status(), 422, "{count} installments");
        let body: Value = response.json().await.unwrap();
        assert_eq!(
            body["error"]["fields"]["installment_count"],
            json!("installment_count_range")
        );
    }
    for count in [2, 48] {
        let response =
            installment_purchase_in(&ana, &nubank, &category, 10_000, count, "2026-01-05").await;
        assert_eq!(response.status(), 201, "{count} installments");
    }
}

#[tokio::test]
async fn the_total_must_be_positive_and_give_at_least_one_cent_per_installment() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let category = category_id(&ana, "Outros", "expense").await;

    for total in [0, -100] {
        let response =
            installment_purchase_in(&ana, &nubank, &category, total, 3, "2026-01-05").await;
        assert_eq!(response.status(), 422);
        let body: Value = response.json().await.unwrap();
        assert_eq!(
            body["error"]["fields"]["total_cents"],
            json!("amount_range")
        );
    }

    // 2 cents cannot be split in 3 installments of at least one cent.
    let response = installment_purchase_in(&ana, &nubank, &category, 2, 3, "2026-01-05").await;
    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["total_cents"],
        json!("total_below_installments")
    );

    // 3 cents is the least that fits in 3 installments.
    let response = installment_purchase_in(&ana, &nubank, &category, 3, 3, "2026-01-05").await;
    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn an_installment_purchase_needs_an_active_expense_category_and_an_active_card() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let salary = category_id(&ana, "Salário", "income").await;

    let response = installment_purchase_in(&ana, &nubank, &salary, 10_000, 3, "2026-01-05").await;
    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["category_id"],
        json!("category_type_mismatch")
    );

    let category = category_id(&ana, "Outros", "expense").await;
    let unknown = "00000000-0000-0000-0000-000000000000";
    let response = installment_purchase_in(&ana, unknown, &category, 10_000, 3, "2026-01-05").await;
    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["credit_card_id"],
        json!("credit_card_not_found")
    );

    let deactivated = ana
        .post_empty(&format!("/credit-cards/{nubank}/deactivate"))
        .await;
    assert_eq!(deactivated.status(), 200);
    let response = installment_purchase_in(&ana, &nubank, &category, 10_000, 3, "2026-01-05").await;
    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["credit_card_id"],
        json!("credit_card_inactive")
    );
}

#[tokio::test]
async fn an_installment_purchase_cannot_start_in_a_paid_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let category = category_id(&ana, "Outros", "expense").await;
    app.set_today(2026, 1, 17); // January's invoice is paid from its due day

    let response = installment_purchase_in(&ana, &nubank, &category, 10_000, 3, "2026-01-05").await;

    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["fields"]["date"], json!("invoice_paid"));
}

#[tokio::test]
async fn the_installment_purchase_requires_a_session() {
    let app = TestApp::spawn().await;
    let response = app
        .session()
        .post_json("/installment-purchases", &json!({}))
        .await;
    assert_eq!(response.status(), 401);
}
