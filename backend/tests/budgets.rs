//! Monthly Budgets and the Budget Alert (UC12, UC13), through the HTTP API.
//!
//! The Alert is never stored: every read of `GET /budgets/status` derives the
//! spending of each Category in the Reference Month again (ADR-0002), so the
//! tests move "today" with `set_today` and read again.
//!
//! The cards used close on day 20 and are due on day 27: a purchase made on or
//! before the 20th of a month is in the invoice due on the 27th of that month,
//! a later one in the invoice due on the 27th of the next month. "Today" starts
//! at 2026-01-15.

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

async fn expense_category(browser: &Session, name: &str) -> String {
    category_id(browser, name, "expense").await
}

async fn set_budget(browser: &Session, category_id: &str, limit_cents: i64) -> reqwest::Response {
    browser
        .put_json(
            &format!("/budgets/{category_id}"),
            &json!({ "limit_cents": limit_cents }),
        )
        .await
}

async fn limit(browser: &Session, category_id: &str, limit_cents: i64) {
    let response = set_budget(browser, category_id, limit_cents).await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
}

async fn expense(browser: &Session, category_id: &str, amount_cents: i64, date: &str) {
    let response = browser
        .post_json(
            "/transactions",
            &json!({
                "type": "expense",
                "amount_cents": amount_cents,
                "date": date,
                "description": "gasto",
                "category_id": category_id,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

async fn income(browser: &Session, amount_cents: i64, date: &str) {
    let response = browser
        .post_json(
            "/transactions",
            &json!({
                "type": "income",
                "amount_cents": amount_cents,
                "date": date,
                "description": "salário",
                "category_id": category_id(browser, "Salário", "income").await,
            }),
        )
        .await;
    assert_eq!(response.status(), 201);
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

async fn card_purchase(
    browser: &Session,
    card_id: &str,
    category_id: &str,
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
                "category_id": category_id,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

async fn installments(
    browser: &Session,
    card_id: &str,
    category_id: &str,
    total_cents: i64,
    count: u32,
    date: &str,
) {
    let response = browser
        .post_json(
            "/installment-purchases",
            &json!({
                "credit_card_id": card_id,
                "total_cents": total_cents,
                "installment_count": count,
                "date": date,
                "description": "parcelado",
                "category_id": category_id,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
}

async fn recurring_expense(
    browser: &Session,
    category_id: &str,
    amount_cents: i64,
    day_of_month: u32,
) -> String {
    let response = browser
        .post_json(
            "/recurring-transactions",
            &json!({
                "type": "expense",
                "amount_cents": amount_cents,
                "day_of_month": day_of_month,
                "description": "fixo",
                "category_id": category_id,
            }),
        )
        .await;
    assert_eq!(response.status(), 201, "{}", response.text().await.unwrap());
    let body: Value = response.json().await.unwrap();
    body["recurring_transaction"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn status(browser: &Session, month: &str) -> Value {
    let response = browser.get(&format!("/budgets/status?month={month}")).await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// The status line of one Category, or `None` if it has no Budget that month.
fn line<'a>(status: &'a Value, category_id: &str) -> Option<&'a Value> {
    status["budgets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|line| line["category"]["id"] == json!(category_id))
}

async fn spent(browser: &Session, month: &str, category_id: &str) -> i64 {
    let body = status(browser, month).await;
    line(&body, category_id).expect("the Category has a Budget")["spent_cents"]
        .as_i64()
        .unwrap()
}

async fn error_code(response: reqwest::Response) -> (u16, Value) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    (status, body["error"].clone())
}

// --- The Budget itself (UC12) ------------------------------------------

#[tokio::test]
async fn a_budget_is_a_limit_for_an_expense_category_and_setting_it_again_replaces_it() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;

    let created = set_budget(&ana, &food, 50_000).await;
    assert_eq!(created.status(), 200);
    let body: Value = created.json().await.unwrap();
    assert_eq!(body["budget"]["limit_cents"], json!(50_000));
    assert_eq!(body["budget"]["effective_from"], json!("2026-01"));
    assert_eq!(body["budget"]["category"]["id"], json!(food));
    assert_eq!(body["budget"]["category"]["name"], json!("Alimentação"));

    limit(&ana, &food, 80_000).await;

    let listed: Value = ana.get("/budgets").await.json().await.unwrap();
    let budgets = listed["budgets"].as_array().unwrap();
    assert_eq!(budgets.len(), 1, "one limit per Category");
    assert_eq!(budgets[0]["limit_cents"], json!(80_000));
}

#[tokio::test]
async fn the_limit_must_be_positive_and_the_category_an_active_expense_category_of_the_user() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bruno = signed_up(&app, "bruno@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;

    for bad in [0, -1, 100_000_000_001] {
        let (status, error) = error_code(set_budget(&ana, &food, bad).await).await;
        assert_eq!(status, 422);
        assert_eq!(error["code"], json!("validation_failed"));
        assert_eq!(error["fields"]["limit_cents"], json!("amount_range"));
    }

    let salary = category_id(&ana, "Salário", "income").await;
    let (status, error) = error_code(set_budget(&ana, &salary, 1_000).await).await;
    assert_eq!(status, 422);
    assert_eq!(
        error["fields"]["category_id"],
        json!("category_type_mismatch")
    );

    // Another user's Category does not exist for Ana, and neither does a
    // malformed id.
    let brunos = expense_category(&bruno, "Alimentação").await;
    let (status, error) = error_code(set_budget(&ana, &brunos, 1_000).await).await;
    assert_eq!(status, 422);
    assert_eq!(error["fields"]["category_id"], json!("category_not_found"));
    let (status, _) = error_code(set_budget(&ana, "not-an-id", 1_000).await).await;
    assert_eq!(status, 422);

    // A deactivated Category takes no new Budget.
    let leisure = expense_category(&ana, "Lazer").await;
    ana.post_empty(&format!("/categories/{leisure}/deactivate"))
        .await;
    let (status, error) = error_code(set_budget(&ana, &leisure, 1_000).await).await;
    assert_eq!(status, 422);
    assert_eq!(error["fields"]["category_id"], json!("category_inactive"));
}

#[tokio::test]
async fn budgets_are_private_to_their_user_and_need_a_session() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bruno = signed_up(&app, "bruno@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;
    limit(&ana, &food, 50_000).await;

    let brunos: Value = bruno.get("/budgets").await.json().await.unwrap();
    assert_eq!(brunos["budgets"], json!([]));
    let brunos: Value = status(&bruno, "2026-01").await;
    assert_eq!(brunos["budgets"], json!([]));
    let (code, error) = error_code(bruno.delete(&format!("/budgets/{food}")).await).await;
    assert_eq!(code, 404);
    assert_eq!(error["code"], json!("budget_not_found"));

    let signed_out = app.session();
    for path in ["/budgets", "/budgets/status"] {
        assert_eq!(signed_out.get(path).await.status(), 401);
    }
    assert_eq!(set_budget(&signed_out, &food, 1_000).await.status(), 401);
    assert_eq!(
        signed_out
            .delete(&format!("/budgets/{food}"))
            .await
            .status(),
        401
    );
}

#[tokio::test]
async fn removing_a_budget_stops_it_from_this_month_on_and_removing_none_is_not_found() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;

    let (status_code, error) = error_code(ana.delete(&format!("/budgets/{food}")).await).await;
    assert_eq!(status_code, 404);
    assert_eq!(error["code"], json!("budget_not_found"));

    limit(&ana, &food, 50_000).await;
    assert_eq!(ana.delete(&format!("/budgets/{food}")).await.status(), 204);

    let listed: Value = ana.get("/budgets").await.json().await.unwrap();
    assert_eq!(listed["budgets"], json!([]));
    assert!(line(&status(&ana, "2026-01").await, &food).is_none());
    // Already removed.
    assert_eq!(ana.delete(&format!("/budgets/{food}")).await.status(), 404);

    // It can be set again afterwards.
    limit(&ana, &food, 30_000).await;
    let listed: Value = ana.get("/budgets").await.json().await.unwrap();
    assert_eq!(listed["budgets"][0]["limit_cents"], json!(30_000));
}

// --- Spending and the Alert (UC13) --------------------------------------

#[tokio::test]
async fn spending_adds_the_expenses_of_the_category_in_the_month_and_the_alert_comes_only_above_the_limit()
 {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;
    let leisure = expense_category(&ana, "Lazer").await;
    limit(&ana, &food, 50_000).await;

    expense(&ana, &food, 20_000, "2026-01-02").await;
    expense(&ana, &food, 5_000, "2026-01-31").await;
    // Not counted: another Category, another month, an income.
    expense(&ana, &leisure, 99_000, "2026-01-10").await;
    expense(&ana, &food, 77_000, "2025-12-31").await;
    expense(&ana, &food, 66_000, "2026-02-01").await;
    income(&ana, 300_000, "2026-01-05").await;

    let body = status(&ana, "2026-01").await;
    assert_eq!(body["month"], json!("2026-01"));
    let food_line = line(&body, &food).unwrap();
    assert_eq!(food_line["limit_cents"], json!(50_000));
    assert_eq!(food_line["spent_cents"], json!(25_000));
    assert_eq!(food_line["remaining_cents"], json!(25_000));
    assert_eq!(food_line["percent_used"], json!(50));
    assert_eq!(food_line["exceeded"], json!(false));
    assert_eq!(body["exceeded_count"], json!(0));
    assert!(line(&body, &leisure).is_none(), "no Budget, no line");

    // Exactly at the limit is not above it.
    expense(&ana, &food, 25_000, "2026-01-20").await;
    let body = status(&ana, "2026-01").await;
    let food_line = line(&body, &food).unwrap();
    assert_eq!(food_line["spent_cents"], json!(50_000));
    assert_eq!(food_line["percent_used"], json!(100));
    assert_eq!(food_line["exceeded"], json!(false));

    // One cent above is.
    expense(&ana, &food, 1, "2026-01-21").await;
    let body = status(&ana, "2026-01").await;
    let food_line = line(&body, &food).unwrap();
    assert_eq!(food_line["spent_cents"], json!(50_001));
    assert_eq!(food_line["remaining_cents"], json!(-1));
    assert_eq!(food_line["exceeded"], json!(true));
    assert_eq!(body["exceeded_count"], json!(1));
}

#[tokio::test]
async fn the_alert_is_calculated_on_read_and_follows_edits_and_deletes() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;
    let leisure = expense_category(&ana, "Lazer").await;
    limit(&ana, &food, 10_000).await;
    expense(&ana, &food, 15_000, "2026-01-05").await;
    assert_eq!(status(&ana, "2026-01").await["exceeded_count"], json!(1));

    let listed: Value = ana
        .get("/transactions?type=expense")
        .await
        .json()
        .await
        .unwrap();
    let id = listed["items"][0]["id"].as_str().unwrap().to_string();

    // Moving the expense to another Category clears the Alert.
    let changed = ana
        .patch_json(
            &format!("/transactions/{id}"),
            &json!({ "category_id": leisure }),
        )
        .await;
    assert_eq!(changed.status(), 200);
    assert_eq!(status(&ana, "2026-01").await["exceeded_count"], json!(0));
    assert_eq!(spent(&ana, "2026-01", &food).await, 0);

    // And back, then deleting it clears it too.
    ana.patch_json(
        &format!("/transactions/{id}"),
        &json!({ "category_id": food }),
    )
    .await;
    assert_eq!(status(&ana, "2026-01").await["exceeded_count"], json!(1));
    assert_eq!(
        ana.delete(&format!("/transactions/{id}")).await.status(),
        204
    );
    assert_eq!(status(&ana, "2026-01").await["exceeded_count"], json!(0));
}

#[tokio::test]
async fn projections_count_in_their_month_future_expenses_and_recurring_occurrences() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let housing = expense_category(&ana, "Moradia").await;
    limit(&ana, &housing, 200_000).await;

    // A one-off expense later this month is a Projection and already counts.
    expense(&ana, &housing, 30_000, "2026-01-28").await;
    // A fixed expense on the 5th: January's Occurrence already fell, the
    // following ones are Projections.
    let rent = recurring_expense(&ana, &housing, 150_000, 5).await;

    // The rent's Occurrence of January belongs to January even though its
    // day (the 5th) has passed: the Reference Month is the month of its date.
    assert_eq!(spent(&ana, "2026-01", &housing).await, 180_000);
    assert_eq!(spent(&ana, "2026-02", &housing).await, 150_000);
    assert_eq!(spent(&ana, "2026-06", &housing).await, 150_000);

    // Editing or skipping the Occurrence of one month changes only that month.
    let edited = ana
        .patch_json(
            &format!("/recurring-transactions/{rent}/occurrences/2026-02"),
            &json!({ "amount_cents": 220_000 }),
        )
        .await;
    assert_eq!(edited.status(), 200);
    assert_eq!(
        ana.delete(&format!(
            "/recurring-transactions/{rent}/occurrences/2026-03"
        ))
        .await
        .status(),
        204
    );
    let february = status(&ana, "2026-02").await;
    let february = line(&february, &housing).unwrap();
    assert_eq!(february["spent_cents"], json!(220_000));
    assert_eq!(february["exceeded"], json!(true));
    assert_eq!(spent(&ana, "2026-03", &housing).await, 0);
    assert_eq!(spent(&ana, "2026-04", &housing).await, 150_000);
}

#[tokio::test]
async fn card_purchases_and_installments_count_in_the_due_month_of_their_invoice() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let leisure = expense_category(&ana, "Lazer").await;
    let food = expense_category(&ana, "Alimentação").await;
    limit(&ana, &leisure, 40_000).await;
    limit(&ana, &food, 40_000).await;
    let nubank = card(&ana, "Nubank").await;

    // On or before the closing day (20th): the invoice due on January 27th.
    card_purchase(&ana, &nubank, &leisure, 10_000, "2026-01-10").await;
    card_purchase(&ana, &nubank, &leisure, 5_000, "2026-01-20").await;
    // After it: the invoice due on February 27th, so February's spending, even
    // though the purchase was made in January.
    card_purchase(&ana, &nubank, &leisure, 7_000, "2026-01-21").await;
    // Other Category.
    card_purchase(&ana, &nubank, &food, 1_000, "2026-01-10").await;

    assert_eq!(spent(&ana, "2026-01", &leisure).await, 15_000);
    assert_eq!(spent(&ana, "2026-02", &leisure).await, 7_000);

    // R$ 100,00 in 3 Installments (33,34 + 33,33 + 33,33), the first in the
    // invoice due on January 27th.
    installments(&ana, &nubank, &food, 10_000, 3, "2026-01-12").await;
    assert_eq!(spent(&ana, "2026-01", &food).await, 1_000 + 3_334);
    assert_eq!(spent(&ana, "2026-02", &food).await, 3_333);
    assert_eq!(spent(&ana, "2026-03", &food).await, 3_333);
    assert_eq!(spent(&ana, "2026-04", &food).await, 0);

    // Once the invoices are paid the spending is still where it was.
    app.set_today(2026, 3, 10);
    assert_eq!(spent(&ana, "2026-01", &leisure).await, 15_000);
    assert_eq!(spent(&ana, "2026-02", &leisure).await, 7_000);
}

#[tokio::test]
async fn paying_an_invoice_is_not_spending() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let leisure = expense_category(&ana, "Lazer").await;
    limit(&ana, &leisure, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    card_purchase(&ana, &nubank, &leisure, 30_000, "2026-01-10").await;
    assert_eq!(spent(&ana, "2026-01", &leisure).await, 30_000);

    // An Early Payment of the invoice...
    let paid = ana
        .post_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payments"),
            &json!({ "amount_cents": 30_000 }),
        )
        .await;
    assert_eq!(paid.status(), 201);
    assert_eq!(spent(&ana, "2026-01", &leisure).await, 30_000);

    // ...and the automatic payment of the Due Day leave the spending as it was.
    app.set_today(2026, 1, 27);
    assert_eq!(spent(&ana, "2026-01", &leisure).await, 30_000);
    app.set_today(2026, 2, 27);
    assert_eq!(spent(&ana, "2026-01", &leisure).await, 30_000);
    assert_eq!(spent(&ana, "2026-02", &leisure).await, 0);
}

#[tokio::test]
async fn a_changed_card_purchase_date_moves_its_spending_to_the_new_invoice_month() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let leisure = expense_category(&ana, "Lazer").await;
    limit(&ana, &leisure, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    card_purchase(&ana, &nubank, &leisure, 30_000, "2026-01-10").await;

    let invoice: Value = ana
        .get(&format!("/credit-cards/{nubank}/invoices/2026-01"))
        .await
        .json()
        .await
        .unwrap();
    let id = invoice["purchases"][0]["id"].as_str().unwrap().to_string();
    let moved = ana
        .patch_json(
            &format!("/card-purchases/{id}"),
            &json!({ "date": "2026-01-25" }),
        )
        .await;
    assert_eq!(moved.status(), 200);

    assert_eq!(spent(&ana, "2026-01", &leisure).await, 0);
    assert_eq!(spent(&ana, "2026-02", &leisure).await, 30_000);
}

// --- Month turn ---------------------------------------------------------

#[tokio::test]
async fn the_limit_applies_every_month_and_the_spending_starts_again_each_month() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;
    limit(&ana, &food, 50_000).await;
    expense(&ana, &food, 60_000, "2026-01-10").await;
    expense(&ana, &food, 10_000, "2026-02-03").await;

    let january = status(&ana, "2026-01").await;
    assert_eq!(line(&january, &food).unwrap()["exceeded"], json!(true));

    // Moving to February: the same limit, a fresh month.
    app.set_today(2026, 2, 1);
    let current = status_current(&ana).await;
    assert_eq!(current["month"], json!("2026-02"));
    let february = line(&current, &food).unwrap();
    assert_eq!(february["limit_cents"], json!(50_000));
    assert_eq!(february["spent_cents"], json!(10_000));
    assert_eq!(february["exceeded"], json!(false));
    assert_eq!(current["exceeded_count"], json!(0));

    // It applies to months ahead as well, without being set again.
    assert_eq!(
        line(&status(&ana, "2026-08").await, &food).unwrap()["limit_cents"],
        json!(50_000)
    );
    // January keeps showing what happened.
    assert_eq!(
        line(&status(&ana, "2026-01").await, &food).unwrap()["exceeded"],
        json!(true)
    );
}

async fn status_current(browser: &Session) -> Value {
    let response = browser.get("/budgets/status").await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

#[tokio::test]
async fn changing_the_limit_applies_from_the_current_month_and_leaves_past_months_as_they_were() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;
    limit(&ana, &food, 50_000).await;
    expense(&ana, &food, 60_000, "2026-01-10").await;
    expense(&ana, &food, 60_000, "2026-02-10").await;

    app.set_today(2026, 2, 1);
    limit(&ana, &food, 100_000).await;

    // January was over its R$ 500,00; February is within the new R$ 1.000,00.
    let january = status(&ana, "2026-01").await;
    let january = line(&january, &food).unwrap();
    assert_eq!(january["limit_cents"], json!(50_000));
    assert_eq!(january["exceeded"], json!(true));
    let february = status(&ana, "2026-02").await;
    let february = line(&february, &food).unwrap();
    assert_eq!(february["limit_cents"], json!(100_000));
    assert_eq!(february["exceeded"], json!(false));

    let listed: Value = ana.get("/budgets").await.json().await.unwrap();
    assert_eq!(listed["budgets"][0]["limit_cents"], json!(100_000));
    assert_eq!(listed["budgets"][0]["effective_from"], json!("2026-02"));

    // Removing it in March stops it from March on and keeps the past.
    app.set_today(2026, 3, 1);
    assert_eq!(ana.delete(&format!("/budgets/{food}")).await.status(), 204);
    assert!(line(&status(&ana, "2026-03").await, &food).is_none());
    assert!(line(&status(&ana, "2026-09").await, &food).is_none());
    assert_eq!(
        line(&status(&ana, "2026-02").await, &food).unwrap()["limit_cents"],
        json!(100_000)
    );
    assert_eq!(
        line(&status(&ana, "2026-01").await, &food).unwrap()["limit_cents"],
        json!(50_000)
    );
}

#[tokio::test]
async fn a_month_before_the_budget_was_defined_has_none() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;
    expense(&ana, &food, 60_000, "2025-12-10").await;
    limit(&ana, &food, 50_000).await;

    assert!(line(&status(&ana, "2025-12").await, &food).is_none());
    assert!(line(&status(&ana, "2026-01").await, &food).is_some());
}

#[tokio::test]
async fn the_month_must_be_a_valid_year_and_month() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    for bad in ["2026-13", "2026-1", "202601", "january", "2026-01-15", ""] {
        let response = ana.get(&format!("/budgets/status?month={bad}")).await;
        let (code, error) = error_code(response).await;
        assert_eq!(code, 400, "month={bad}");
        assert_eq!(error["code"], json!("invalid_request"));
    }
}

// --- Deactivated Category -------------------------------------------------

#[tokio::test]
async fn the_budget_of_a_deactivated_category_is_kept_without_an_alert_and_applies_again_on_reactivation()
 {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let leisure = expense_category(&ana, "Lazer").await;
    limit(&ana, &leisure, 10_000).await;
    expense(&ana, &leisure, 15_000, "2026-01-10").await;
    assert_eq!(status(&ana, "2026-01").await["exceeded_count"], json!(1));

    let deactivated = ana
        .post_empty(&format!("/categories/{leisure}/deactivate"))
        .await;
    assert_eq!(deactivated.status(), 200);

    // Kept: still listed, with the Category marked as deactivated.
    let listed: Value = ana.get("/budgets").await.json().await.unwrap();
    assert_eq!(listed["budgets"][0]["limit_cents"], json!(10_000));
    assert_eq!(listed["budgets"][0]["category"]["active"], json!(false));
    // No Alert, although the spending is above the limit.
    let body = status(&ana, "2026-01").await;
    let kept = line(&body, &leisure).unwrap();
    assert_eq!(kept["category"]["active"], json!(false));
    assert_eq!(kept["spent_cents"], json!(15_000));
    assert_eq!(kept["exceeded"], json!(false));
    assert_eq!(body["exceeded_count"], json!(0));

    // Reactivated: it applies again, with the same limit.
    let reactivated = ana
        .post_empty(&format!("/categories/{leisure}/reactivate"))
        .await;
    assert_eq!(reactivated.status(), 200);
    let body = status(&ana, "2026-01").await;
    let back = line(&body, &leisure).unwrap();
    assert_eq!(back["limit_cents"], json!(10_000));
    assert_eq!(back["exceeded"], json!(true));
    assert_eq!(body["exceeded_count"], json!(1));
}

#[tokio::test]
async fn the_budget_of_a_deactivated_category_can_still_be_removed() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let leisure = expense_category(&ana, "Lazer").await;
    limit(&ana, &leisure, 10_000).await;
    ana.post_empty(&format!("/categories/{leisure}/deactivate"))
        .await;

    assert_eq!(
        ana.delete(&format!("/budgets/{leisure}")).await.status(),
        204
    );
    let listed: Value = ana.get("/budgets").await.json().await.unwrap();
    assert_eq!(listed["budgets"], json!([]));
}

// --- Charges, Carried Balance and Installment Anticipation ---------------

#[tokio::test]
async fn charges_count_in_the_charges_category_by_the_due_month_of_their_invoice() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let charges = expense_category(&ana, "Juros e encargos").await;
    limit(&ana, &charges, 5_000).await;
    let nubank = card(&ana, "Nubank").await;

    // Recorded in the invoice closing in January (due January 27th); the day
    // the bank charged it does not matter.
    let recorded = ana
        .post_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/charges"),
            &json!({ "amount_cents": 6_000, "date": "2025-12-30" }),
        )
        .await;
    assert_eq!(recorded.status(), 201, "{}", recorded.text().await.unwrap());

    let body = status(&ana, "2026-01").await;
    let line = line(&body, &charges).unwrap();
    assert_eq!(line["spent_cents"], json!(6_000));
    assert_eq!(line["exceeded"], json!(true));
    assert_eq!(spent(&ana, "2026-02", &charges).await, 0);
    assert_eq!(spent(&ana, "2025-12", &charges).await, 0);
}

#[tokio::test]
async fn the_carried_balance_and_the_corrected_payment_are_not_spending() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let leisure = expense_category(&ana, "Lazer").await;
    limit(&ana, &leisure, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    card_purchase(&ana, &nubank, &leisure, 30_000, "2026-01-10").await;

    // The due date comes and only R$ 100,00 of R$ 300,00 were really paid:
    // R$ 200,00 are carried to the next invoice.
    app.set_today(2026, 1, 27);
    let corrected = ana
        .put_json(
            &format!("/credit-cards/{nubank}/invoices/2026-01/payment"),
            &json!({ "amount_cents": 10_000 }),
        )
        .await;
    assert_eq!(
        corrected.status(),
        200,
        "{}",
        corrected.text().await.unwrap()
    );

    assert_eq!(spent(&ana, "2026-01", &leisure).await, 30_000);
    // The next invoice (due February 27th) now totals R$ 200,00 but none of
    // it is spending.
    assert_eq!(spent(&ana, "2026-02", &leisure).await, 0);
}

#[tokio::test]
async fn anticipated_installments_count_in_the_open_invoices_month_with_their_discount() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let food = expense_category(&ana, "Alimentação").await;
    limit(&ana, &food, 100_000).await;
    let nubank = card(&ana, "Nubank").await;
    // R$ 300,00 in 3: R$ 100,00 in the invoices due in January, February and
    // March.
    installments(&ana, &nubank, &food, 30_000, 3, "2026-01-12").await;
    assert_eq!(spent(&ana, "2026-02", &food).await, 10_000);

    let anticipable: Value = ana
        .get(&format!("/credit-cards/{nubank}/anticipable-installments"))
        .await
        .json()
        .await
        .unwrap();
    let ids: Vec<String> = anticipable["installments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids.len(), 2);
    let anticipated = ana
        .post_json(
            &format!("/credit-cards/{nubank}/installment-anticipations"),
            &json!({ "installment_ids": ids, "discount_cents": 1_000, "confirm": true }),
        )
        .await;
    assert_eq!(
        anticipated.status(),
        200,
        "{}",
        anticipated.text().await.unwrap()
    );

    // R$ 100,00 + (R$ 200,00 - R$ 10,00 of discount) in the January invoice.
    assert_eq!(spent(&ana, "2026-01", &food).await, 29_000);
    assert_eq!(spent(&ana, "2026-02", &food).await, 0);
    assert_eq!(spent(&ana, "2026-03", &food).await, 0);
}
