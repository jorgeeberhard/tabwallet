//! Installment Anticipation with an optional Discount (UC19), through the HTTP
//! API: bringing future Installments of a card to its open invoice, the
//! proportional split of the Discount, the irreversibility, and how an
//! anticipated Installment behaves in the invoices, the balance, the card's
//! days and the edits of its purchase.
//!
//! Unless a test says otherwise "today" is 2026-01-15 and the cards close on
//! the 10th and are due on the 17th, so the open invoice is the one closing
//! on 2026-02-10 (due 2026-02-17, Reference Month 2026-02).

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

async fn category_id(browser: &Session, name: &str) -> String {
    let body: Value = browser.get("/categories").await.json().await.unwrap();
    body["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|category| category["name"] == json!(name) && category["type"] == json!("expense"))
        .unwrap_or_else(|| panic!("no expense category named {name}"))["id"]
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

/// Launches an installment purchase in "Outros" and returns it.
async fn purchase(
    browser: &Session,
    card_id: &str,
    total_cents: i64,
    count: i64,
    date: &str,
) -> Value {
    purchase_in(browser, card_id, "Outros", total_cents, count, date).await
}

async fn purchase_in(
    browser: &Session,
    card_id: &str,
    category: &str,
    total_cents: i64,
    count: i64,
    date: &str,
) -> Value {
    let category = category_id(browser, category).await;
    let response = browser
        .post_json(
            "/installment-purchases",
            &json!({
                "credit_card_id": card_id,
                "total_cents": total_cents,
                "installment_count": count,
                "date": date,
                "description": "geladeira",
                "category_id": category,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    body["installment_purchase"].clone()
}

/// The id of the Installment of this number.
fn installment_id(purchase: &Value, number: i64) -> String {
    purchase["installments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|installment| installment["number"] == json!(number))
        .unwrap_or_else(|| panic!("no installment {number}: {purchase}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn anticipate(browser: &Session, card_id: &str, body: &Value) -> reqwest::Response {
    browser
        .post_json(
            &format!("/credit-cards/{card_id}/installment-anticipations"),
            body,
        )
        .await
}

/// Anticipates with confirmation and expects it to work.
async fn anticipate_ok(
    browser: &Session,
    card_id: &str,
    ids: &[String],
    discount_cents: Option<i64>,
) -> Value {
    let mut body = json!({ "installment_ids": ids, "confirm": true });
    if let Some(discount) = discount_cents {
        body["discount_cents"] = json!(discount);
    }
    let response = anticipate(browser, card_id, &body).await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    body["anticipation"].clone()
}

async fn error_of(response: reqwest::Response, status: u16) -> Value {
    assert_eq!(response.status(), status);
    let body: Value = response.json().await.unwrap();
    body["error"].clone()
}

async fn invoices(browser: &Session, card_id: &str) -> Vec<Value> {
    let body: Value = browser
        .get(&format!("/credit-cards/{card_id}/invoices"))
        .await
        .json()
        .await
        .unwrap();
    body["invoices"].as_array().unwrap().clone()
}

fn invoice_closing(invoices: &[Value], month: &str) -> Value {
    invoices
        .iter()
        .find(|invoice| invoice["closing_month"] == json!(month))
        .unwrap_or_else(|| panic!("no invoice closing in {month}: {invoices:?}"))
        .clone()
}

async fn statement_purchase(browser: &Session) -> Value {
    let body: Value = browser
        .get("/transactions?type=expense")
        .await
        .json()
        .await
        .unwrap();
    body["items"][0].clone()
}

fn amounts(purchase: &Value) -> Vec<i64> {
    purchase["installments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|installment| installment["amount_cents"].as_i64().unwrap())
        .collect()
}

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

async fn current_balance(browser: &Session) -> i64 {
    let body: Value = browser.get("/balance").await.json().await.unwrap();
    body["current_balance_cents"].as_i64().unwrap()
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

// --- bringing Installments to the open invoice ---------------------------

#[tokio::test]
async fn chosen_installments_move_to_the_open_invoice_and_the_others_stay() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    // February, March, April and May: 25,00 each.
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let ids = [installment_id(&bought, 3), installment_id(&bought, 4)];

    let anticipation = anticipate_ok(&ana, &nubank, &ids, None).await;

    assert_eq!(anticipation["original_total_cents"], json!(5_000));
    assert_eq!(anticipation["discount_cents"], json!(0));
    assert_eq!(anticipation["total_cents"], json!(5_000));
    // The invoice that received them, as it stands now.
    assert_eq!(anticipation["invoice"]["closing_month"], json!("2026-02"));
    assert_eq!(anticipation["invoice"]["total_cents"], json!(7_500));
    let moved = anticipation["installments"].as_array().unwrap();
    assert_eq!(moved.len(), 2);
    assert_eq!(moved[0]["number"], json!(3));
    assert_eq!(moved[0]["anticipated"], json!(true));
    assert_eq!(moved[0]["original_invoice_month"], json!("2026-04"));
    assert_eq!(moved[0]["invoice"]["closing_month"], json!("2026-02"));

    let listed = invoices(&ana, &nubank).await;
    assert_eq!(
        invoice_closing(&listed, "2026-02")["total_cents"],
        json!(7_500)
    );
    assert_eq!(
        invoice_closing(&listed, "2026-03")["total_cents"],
        json!(2_500)
    );
    // April's and May's invoices have nothing left.
    assert!(
        listed
            .iter()
            .all(|invoice| invoice["closing_month"] != json!("2026-04")
                && invoice["closing_month"] != json!("2026-05")),
        "{listed:?}"
    );
    let item = statement_purchase(&ana).await;
    assert_eq!(item["has_anticipated_installments"], json!(true));
    assert_eq!(
        invoice_months(&item),
        vec!["2026-02", "2026-03", "2026-02", "2026-02"]
    );
    assert_eq!(item["installments"][0]["anticipated"], json!(false));
    assert_eq!(item["installments"][2]["anticipated"], json!(true));
}

#[tokio::test]
async fn anticipated_installments_count_in_the_reference_month_of_the_open_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    // The last Installment would count in May (the invoice due 2026-05-17).
    assert_eq!(
        bought["installments"][3]["reference_month"],
        json!("2026-05")
    );

    anticipate_ok(&ana, &nubank, &[installment_id(&bought, 4)], None).await;

    let item = statement_purchase(&ana).await;
    assert_eq!(item["installments"][3]["reference_month"], json!("2026-02"));
    let february: Value = ana
        .get(&format!("/credit-cards/{nubank}/invoices/2026-02"))
        .await
        .json()
        .await
        .unwrap();
    let in_february = february["installments"].as_array().unwrap();
    assert_eq!(in_february.len(), 2);
    assert!(
        in_february
            .iter()
            .all(|installment| installment["reference_month"] == json!("2026-02"))
    );
    let anticipated = in_february
        .iter()
        .find(|installment| installment["number"] == json!(4))
        .unwrap();
    assert_eq!(anticipated["anticipated"], json!(true));
}

#[tokio::test]
async fn installments_of_several_purchases_can_be_anticipated_together() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let geladeira = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await; // 25,00 x 4
    let sofa = purchase(&ana, &nubank, 6_000, 2, "2026-02-05").await; // 30,00 x 2
    let ids = [installment_id(&geladeira, 2), installment_id(&sofa, 2)];

    let anticipation = anticipate_ok(&ana, &nubank, &ids, None).await;

    assert_eq!(anticipation["original_total_cents"], json!(5_500));
    let listed = invoices(&ana, &nubank).await;
    // February: 25,00 + 30,00 own Installments + the two brought.
    assert_eq!(
        invoice_closing(&listed, "2026-02")["total_cents"],
        json!(11_000)
    );
    // March has nothing left, so it is not listed; April still has its own.
    assert!(
        listed
            .iter()
            .all(|invoice| invoice["closing_month"] != json!("2026-03")),
        "{listed:?}"
    );
    assert_eq!(
        invoice_closing(&listed, "2026-04")["total_cents"],
        json!(2_500)
    );
}

#[tokio::test]
async fn listing_the_same_installment_twice_counts_it_once() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let id = installment_id(&bought, 3);

    let anticipation = anticipate_ok(&ana, &nubank, &[id.clone(), id], None).await;

    assert_eq!(anticipation["installments"].as_array().unwrap().len(), 1);
    assert_eq!(anticipation["original_total_cents"], json!(2_500));
}

#[tokio::test]
async fn the_anticipable_installments_are_those_in_invoices_after_the_open_one() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    // January (closed), February (open), March, April.
    purchase(&ana, &nubank, 9_000, 4, "2026-01-05").await;

    let response = ana
        .get(&format!("/credit-cards/{nubank}/anticipable-installments"))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["open_invoice"]["closing_month"], json!("2026-02"));
    assert_eq!(body["open_invoice"]["state"], json!("open"));
    let listed = body["installments"].as_array().unwrap();
    let numbers: Vec<i64> = listed
        .iter()
        .map(|installment| installment["number"].as_i64().unwrap())
        .collect();
    assert_eq!(numbers, vec![3, 4]);
    assert_eq!(listed[0]["amount_cents"], json!(2_250));
    assert_eq!(listed[0]["installment_count"], json!(4));
    assert_eq!(listed[0]["invoice"]["closing_month"], json!("2026-03"));
    assert_eq!(listed[0]["description"], json!("geladeira"));
    assert_eq!(listed[0]["category"]["name"], json!("Outros"));
}

#[tokio::test]
async fn an_anticipated_installment_leaves_the_anticipable_list() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    anticipate_ok(&ana, &nubank, &[installment_id(&bought, 4)], None).await;

    let body: Value = ana
        .get(&format!("/credit-cards/{nubank}/anticipable-installments"))
        .await
        .json()
        .await
        .unwrap();

    let numbers: Vec<i64> = body["installments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|installment| installment["number"].as_i64().unwrap())
        .collect();
    assert_eq!(numbers, vec![2, 3]);
}

// --- the Discount ----------------------------------------------------------

#[tokio::test]
async fn the_discount_is_split_proportionally_and_reduces_each_installment() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let geladeira = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await; // 25,00 x 4
    let sofa = purchase(&ana, &nubank, 6_000, 2, "2026-02-05").await; // 30,00 x 2
    // 25,00 + 30,00 + 25,00 = 80,00 with a 10,00 Discount: 3,125 + 3,75 +
    // 3,125 each, the leftover cent going to the earliest of the tie.
    let ids = [
        installment_id(&geladeira, 2),
        installment_id(&sofa, 2),
        installment_id(&geladeira, 3),
    ];

    let anticipation = anticipate_ok(&ana, &nubank, &ids, Some(1_000)).await;

    assert_eq!(anticipation["original_total_cents"], json!(8_000));
    assert_eq!(anticipation["discount_cents"], json!(1_000));
    assert_eq!(anticipation["total_cents"], json!(7_000));
    let moved = anticipation["installments"].as_array().unwrap();
    let shares: Vec<(i64, i64, i64, i64)> = moved
        .iter()
        .map(|installment| {
            (
                installment["number"].as_i64().unwrap(),
                installment["original_amount_cents"].as_i64().unwrap(),
                installment["discount_cents"].as_i64().unwrap(),
                installment["amount_cents"].as_i64().unwrap(),
            )
        })
        .collect();
    // Ordered by invoice, purchase date and number: geladeira 2, sofa 2,
    // geladeira 3 (the two in March come first).
    assert_eq!(
        shares,
        vec![
            (2, 2_500, 313, 2_187),
            (2, 3_000, 375, 2_625),
            (3, 2_500, 312, 2_188),
        ]
    );
    let discount: i64 = shares.iter().map(|share| share.2).sum();
    assert_eq!(discount, 1_000);
    // The February invoice: its own Installments (25,00 + 30,00) + 70,00.
    let listed = invoices(&ana, &nubank).await;
    assert_eq!(
        invoice_closing(&listed, "2026-02")["total_cents"],
        json!(12_500)
    );
}

#[tokio::test]
async fn the_expense_of_the_original_purchase_is_reduced_by_its_share_of_the_discount() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase_in(&ana, &nubank, "Lazer", 10_000, 4, "2026-02-05").await;
    let ids = [installment_id(&bought, 3), installment_id(&bought, 4)];

    anticipate_ok(&ana, &nubank, &ids, Some(500)).await;

    let item = statement_purchase(&ana).await;
    // The purchase stays in its own Category, now with less to pay.
    assert_eq!(item["category"]["name"], json!("Lazer"));
    assert_eq!(amounts(&item), vec![2_500, 2_500, 2_250, 2_250]);
    assert_eq!(item["discount_cents"], json!(500));
    assert_eq!(
        item["installments"][2]["original_amount_cents"],
        json!(2_500)
    );
    assert_eq!(item["installments"][2]["discount_cents"], json!(250));
    // What the user informed is kept as a record.
    assert_eq!(item["total_cents"], json!(10_000));
}

#[tokio::test]
async fn a_single_cent_of_discount_goes_to_the_earliest_installment() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    // February 33,34, March 33,33, April 33,33.
    let bought = purchase(&ana, &nubank, 10_000, 3, "2026-02-05").await;
    let ids = [installment_id(&bought, 3), installment_id(&bought, 2)];

    let anticipation = anticipate_ok(&ana, &nubank, &ids, Some(1)).await;

    let discounts: Vec<i64> = anticipation["installments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|installment| installment["discount_cents"].as_i64().unwrap())
        .collect();
    // Installment 2 (March) is in an earlier invoice than 3 (April).
    assert_eq!(discounts, vec![1, 0]);
}

#[tokio::test]
async fn the_discount_is_optional_but_when_given_it_must_be_positive() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let ids = [installment_id(&bought, 3)];

    for discount in [0, -50] {
        let response = anticipate(
            &ana,
            &nubank,
            &json!({ "installment_ids": ids, "discount_cents": discount, "confirm": true }),
        )
        .await;
        let error = error_of(response, 422).await;
        assert_eq!(error["code"], json!("validation_failed"));
        assert_eq!(
            error["fields"]["discount_cents"],
            json!("discount_range"),
            "{discount}"
        );
    }
    let without = anticipate(
        &ana,
        &nubank,
        &json!({ "installment_ids": ids, "discount_cents": null, "confirm": true }),
    )
    .await;
    assert_eq!(without.status(), 200);
}

#[tokio::test]
async fn the_discount_never_reaches_the_total_of_the_chosen_installments() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let ids = [installment_id(&bought, 3), installment_id(&bought, 4)];

    // More than the 50,00 chosen, exactly the 50,00 and 49,99: an Installment
    // cannot be free, so each must keep at least one cent.
    for discount in [5_001, 5_000, 4_999] {
        let response = anticipate(
            &ana,
            &nubank,
            &json!({ "installment_ids": ids, "discount_cents": discount, "confirm": true }),
        )
        .await;
        let error = error_of(response, 422).await;
        assert_eq!(
            error["fields"]["discount_cents"],
            json!("discount_exceeds_total"),
            "{discount}"
        );
    }
    // 49,98 leaves one cent in each.
    let response = anticipate(
        &ana,
        &nubank,
        &json!({ "installment_ids": ids, "discount_cents": 4_998, "confirm": true }),
    )
    .await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["anticipation"]["total_cents"], json!(2));
    assert_eq!(
        amounts(&statement_purchase(&ana).await),
        vec![2_500, 2_500, 1, 1]
    );
}

// --- confirmation and irreversibility ---------------------------------------

#[tokio::test]
async fn the_api_refuses_to_anticipate_without_an_explicit_confirmation() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let ids = [installment_id(&bought, 4)];

    for body in [
        json!({ "installment_ids": ids }),
        json!({ "installment_ids": ids, "confirm": false }),
        json!({ "installment_ids": ids, "discount_cents": 100, "confirm": false }),
    ] {
        let response = anticipate(&ana, &nubank, &body).await;
        let error = error_of(response, 422).await;
        assert_eq!(error["code"], json!("validation_failed"), "{body}");
        assert_eq!(
            error["fields"]["confirm"],
            json!("confirmation_required"),
            "{body}"
        );
    }

    // Nothing moved.
    let item = statement_purchase(&ana).await;
    assert_eq!(
        invoice_months(&item),
        vec!["2026-02", "2026-03", "2026-04", "2026-05"]
    );
    assert_eq!(amounts(&item), vec![2_500; 4]);
}

#[tokio::test]
async fn at_least_one_installment_must_be_chosen() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;

    for body in [
        json!({ "installment_ids": [], "confirm": true }),
        json!({ "confirm": true }),
    ] {
        let response = anticipate(&ana, &nubank, &body).await;
        let error = error_of(response, 422).await;
        assert_eq!(
            error["fields"]["installment_ids"],
            json!("installments_required"),
            "{body}"
        );
    }
}

#[tokio::test]
async fn there_is_no_way_to_undo_an_anticipation() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let id = installment_id(&bought, 4);
    anticipate_ok(&ana, &nubank, std::slice::from_ref(&id), Some(100)).await;

    // The anticipated Installment cannot be anticipated again, and there is
    // no route that would give it back.
    let again = anticipate(
        &ana,
        &nubank,
        &json!({ "installment_ids": [id], "confirm": true }),
    )
    .await;
    assert_eq!(
        error_of(again, 409).await["code"],
        json!("installments_not_anticipable")
    );
    for path in [
        format!("/credit-cards/{nubank}/installment-anticipations"),
        format!("/credit-cards/{nubank}/installment-anticipations/{id}"),
        format!("/installments/{id}/anticipation"),
    ] {
        let deleted = ana.delete(&path).await;
        assert!(
            deleted.status() == 404 || deleted.status() == 405,
            "{path}: {}",
            deleted.status()
        );
    }
    let item = statement_purchase(&ana).await;
    assert_eq!(item["installments"][3]["anticipated"], json!(true));
    assert_eq!(item["installments"][3]["amount_cents"], json!(2_400));
}

// --- what can be anticipated -----------------------------------------------

#[tokio::test]
async fn only_installments_in_invoices_after_the_open_one_can_be_anticipated() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    // January (closed, due on the 17th), February (open), March, April.
    let bought = purchase(&ana, &nubank, 9_000, 4, "2026-01-05").await;

    for number in [1, 2] {
        let response = anticipate(
            &ana,
            &nubank,
            &json!({ "installment_ids": [installment_id(&bought, number)], "confirm": true }),
        )
        .await;
        let error = error_of(response, 409).await;
        assert_eq!(
            error["code"],
            json!("installments_not_anticipable"),
            "{number}"
        );
    }

    // After the due date January's invoice is paid: still not anticipable.
    app.set_today(2026, 1, 20);
    let response = anticipate(
        &ana,
        &nubank,
        &json!({ "installment_ids": [installment_id(&bought, 1)], "confirm": true }),
    )
    .await;
    assert_eq!(
        error_of(response, 409).await["code"],
        json!("installments_not_anticipable")
    );
}

#[tokio::test]
async fn one_installment_that_cannot_be_anticipated_stops_the_whole_operation() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    // Installment 1 is already in the open invoice.
    let ids = [installment_id(&bought, 4), installment_id(&bought, 1)];

    let response = anticipate(
        &ana,
        &nubank,
        &json!({ "installment_ids": ids, "discount_cents": 100, "confirm": true }),
    )
    .await;

    assert_eq!(
        error_of(response, 409).await["code"],
        json!("installments_not_anticipable")
    );
    let item = statement_purchase(&ana).await;
    assert_eq!(amounts(&item), vec![2_500; 4]);
    assert_eq!(
        invoice_months(&item),
        vec!["2026-02", "2026-03", "2026-04", "2026-05"]
    );
}

#[tokio::test]
async fn once_the_open_invoice_closes_its_anticipated_installments_are_history() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let anticipated = installment_id(&bought, 4);
    anticipate_ok(&ana, &nubank, std::slice::from_ref(&anticipated), None).await;

    // February's invoice closed; March's is the open one now.
    app.set_today(2026, 2, 11);

    let response = anticipate(
        &ana,
        &nubank,
        &json!({ "installment_ids": [anticipated], "confirm": true }),
    )
    .await;
    assert_eq!(
        error_of(response, 409).await["code"],
        json!("installments_not_anticipable")
    );
    // Installment 3 (April) can go to March now.
    let moved = anticipate_ok(&ana, &nubank, &[installment_id(&bought, 3)], None).await;
    assert_eq!(moved["invoice"]["closing_month"], json!("2026-03"));
    assert_eq!(moved["invoice"]["total_cents"], json!(5_000));
}

#[tokio::test]
async fn installments_must_belong_to_the_card_and_to_the_user() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let inter = card(&ana, "Inter", 10, 17).await;
    let bias = card(&bia, "Nubank", 10, 17).await;
    let on_nubank = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let on_inter = purchase(&ana, &inter, 10_000, 4, "2026-02-05").await;
    let on_bias = purchase(&bia, &bias, 10_000, 4, "2026-02-05").await;

    // An Installment of another card of the same user.
    let response = anticipate(
        &ana,
        &nubank,
        &json!({
            "installment_ids": [installment_id(&on_nubank, 3), installment_id(&on_inter, 3)],
            "confirm": true,
        }),
    )
    .await;
    let error = error_of(response, 422).await;
    assert_eq!(
        error["fields"]["installment_ids"],
        json!("installment_not_found")
    );
    // An Installment of another user, and a card of another user.
    let response = anticipate(
        &ana,
        &nubank,
        &json!({ "installment_ids": [installment_id(&on_bias, 3)], "confirm": true }),
    )
    .await;
    assert_eq!(
        error_of(response, 422).await["fields"]["installment_ids"],
        json!("installment_not_found")
    );
    let response = anticipate(
        &ana,
        &bias,
        &json!({ "installment_ids": [installment_id(&on_nubank, 3)], "confirm": true }),
    )
    .await;
    assert_eq!(
        error_of(response, 404).await["code"],
        json!("credit_card_not_found")
    );
    // Nothing moved.
    let item = statement_purchase(&ana).await;
    assert_eq!(item["has_anticipated_installments"], json!(false));
    let listed: Value = bia
        .get(&format!("/credit-cards/{bias}/anticipable-installments"))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(listed["installments"].as_array().unwrap().len(), 3);
    let foreign = ana
        .get(&format!("/credit-cards/{bias}/anticipable-installments"))
        .await;
    assert_eq!(foreign.status(), 404);
}

#[tokio::test]
async fn anticipating_requires_a_session() {
    let app = TestApp::spawn().await;
    let stranger = app.session();
    let id = "00000000-0000-0000-0000-000000000000";

    let post = anticipate(
        &stranger,
        id,
        &json!({ "installment_ids": [id], "confirm": true }),
    )
    .await;
    assert_eq!(post.status(), 401);
    let get = stranger
        .get(&format!("/credit-cards/{id}/anticipable-installments"))
        .await;
    assert_eq!(get.status(), 401);
}

// --- balance and projection ------------------------------------------------

#[tokio::test]
async fn the_balance_and_the_projection_follow_the_discounted_invoice() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let opening = ana
        .post_json("/balance/opening", &json!({ "amount_cents": 100_000 }))
        .await;
    assert_eq!(opening.status(), 201);
    let nubank = card(&ana, "Nubank", 10, 17).await;
    // 25,00 in February, March, April and May.
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let before = projected_at(&ana, "2026-05-31").await;
    assert_eq!(before, 100_000 - 10_000);

    // Bring April's and May's to February with a 10,00 Discount.
    let ids = [installment_id(&bought, 3), installment_id(&bought, 4)];
    anticipate_ok(&ana, &nubank, &ids, Some(1_000)).await;

    // Nothing left the balance by the operation itself.
    assert_eq!(current_balance(&ana).await, 100_000);
    // The purchase now costs 90,00. February's invoice takes 25,00 of its
    // own plus the two brought, 20,00 each after the Discount.
    assert_eq!(
        projected_at(&ana, "2026-02-28").await,
        100_000 - 2_500 - 4_000
    );
    assert_eq!(projected_at(&ana, "2026-05-31").await, 100_000 - 9_000);
    app.set_today(2026, 2, 17);
    assert_eq!(current_balance(&ana).await, 100_000 - 6_500);
    app.set_today(2026, 3, 17);
    assert_eq!(current_balance(&ana).await, 100_000 - 9_000);
}

// --- the card's days, edits and cancellation ----------------------------------

#[tokio::test]
async fn new_card_days_move_the_other_installments_but_not_the_anticipated_one() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    anticipate_ok(&ana, &nubank, &[installment_id(&bought, 4)], Some(100)).await;

    // With the Closing Day on the 3rd, a purchase of the 5th enters the
    // invoice of the next month: the three Installments left move one month.
    let changed = ana
        .patch_json(
            &format!("/credit-cards/{nubank}"),
            &json!({ "closing_day": 3 }),
        )
        .await;
    assert_eq!(changed.status(), 200);

    let item = statement_purchase(&ana).await;
    assert_eq!(
        invoice_months(&item),
        vec!["2026-03", "2026-04", "2026-05", "2026-02"]
    );
    assert_eq!(amounts(&item), vec![2_500, 2_500, 2_500, 2_400]);
    assert_eq!(item["installments"][3]["anticipated"], json!(true));
}

#[tokio::test]
async fn a_purchase_with_anticipated_installments_only_lets_the_category_change() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let id = bought["id"].as_str().unwrap().to_string();
    anticipate_ok(&ana, &nubank, &[installment_id(&bought, 4)], Some(100)).await;
    let lazer = category_id(&ana, "Lazer").await;

    for changes in [
        json!({ "total_cents": 20_000 }),
        json!({ "installment_count": 3 }),
        json!({ "date": "2026-02-06" }),
        json!({ "description": "outra" }),
    ] {
        let response = ana
            .patch_json(&format!("/installment-purchases/{id}"), &changes)
            .await;
        let error = error_of(response, 409).await;
        assert_eq!(
            error["code"],
            json!("installments_anticipated"),
            "{changes}"
        );
    }

    let response = ana
        .patch_json(
            &format!("/installment-purchases/{id}"),
            &json!({ "category_id": lazer }),
        )
        .await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    let edited = &body["installment_purchase"];
    assert_eq!(edited["category"]["name"], json!("Lazer"));
    assert_eq!(edited["has_anticipated_installments"], json!(true));
    assert_eq!(amounts(edited), vec![2_500, 2_500, 2_500, 2_400]);
}

#[tokio::test]
async fn deleting_a_purchase_with_anticipated_installments_keeps_them() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let id = bought["id"].as_str().unwrap().to_string();
    anticipate_ok(&ana, &nubank, &[installment_id(&bought, 4)], Some(100)).await;

    let response = ana.delete(&format!("/installment-purchases/{id}")).await;

    // Like paid ones, anticipated Installments are history: the remaining
    // ones are cancelled and the purchase stays with the anticipated one.
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    let cancelled = &body["installment_purchase"];
    assert_eq!(cancelled["cancelled"], json!(true));
    assert_eq!(amounts(cancelled), vec![2_400]);
    assert_eq!(cancelled["installments"][0]["number"], json!(4));
    assert_eq!(invoice_months(cancelled), vec!["2026-02"]);
    let listed = invoices(&ana, &nubank).await;
    assert_eq!(
        invoice_closing(&listed, "2026-02")["total_cents"],
        json!(2_400)
    );

    // Nothing is left to cancel, and the anticipation is not undone.
    let again = ana.delete(&format!("/installment-purchases/{id}")).await;
    assert_eq!(
        error_of(again, 409).await["code"],
        json!("installments_anticipated")
    );
}

#[tokio::test]
async fn deleting_a_purchase_without_anticipated_installments_still_removes_it_whole() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let nubank = card(&ana, "Nubank", 10, 17).await;
    let bought = purchase(&ana, &nubank, 10_000, 4, "2026-02-05").await;
    let other = purchase(&ana, &nubank, 6_000, 2, "2026-02-05").await;
    anticipate_ok(&ana, &nubank, &[installment_id(&other, 2)], None).await;

    let response = ana
        .delete(&format!(
            "/installment-purchases/{}",
            bought["id"].as_str().unwrap()
        ))
        .await;

    assert_eq!(response.status(), 204);
    let listed = invoices(&ana, &nubank).await;
    // Only the second purchase is left: both its Installments in February.
    assert_eq!(
        invoice_closing(&listed, "2026-02")["total_cents"],
        json!(6_000)
    );
}
