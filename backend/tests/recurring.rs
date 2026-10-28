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

async fn change_amount(browser: &Session, id: &str, amount_cents: i64) -> reqwest::Response {
    browser
        .patch_json(
            &format!("/recurring-transactions/{id}"),
            &json!({ "amount_cents": amount_cents }),
        )
        .await
}

#[tokio::test]
async fn a_new_amount_applies_from_the_month_of_the_change_and_never_alters_past_months() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let rent = create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;
    // A one-off edit of June, made before the amount changes.
    let edited = ana
        .patch_json(
            &occurrence_path(&rent, "2026-06"),
            &json!({ "amount_cents": 50_000 }),
        )
        .await;
    assert_eq!(edited.status(), 200);

    // In March the 5th has already passed, yet March gets the new amount.
    app.set_today(2026, 3, 10);
    let response = change_amount(&ana, &rent, 120_000).await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["recurring_transaction"]["amount_cents"],
        json!(120_000)
    );

    assert_eq!(
        months_between(&ana, "2026-01-01", "2026-07-31").await,
        vec![
            ("2026-01".to_string(), 100_000, false),
            ("2026-02".to_string(), 100_000, false),
            ("2026-03".to_string(), 120_000, false),
            ("2026-04".to_string(), 120_000, false),
            ("2026-05".to_string(), 120_000, false),
            // The edit of that single month is not overwritten.
            ("2026-06".to_string(), 50_000, true),
            ("2026-07".to_string(), 120_000, false),
        ]
    );
    assert_eq!(current_balance(&ana).await, -320_000); // Jan, Feb, Mar

    // A second change in the same month replaces the first.
    assert_eq!(change_amount(&ana, &rent, 130_000).await.status(), 200);
    assert_eq!(current_balance(&ana).await, -330_000);

    // A later change leaves the months before it alone.
    app.set_today(2026, 5, 20);
    assert_eq!(change_amount(&ana, &rent, 90_000).await.status(), 200);
    let found = months_between(&ana, "2026-03-01", "2026-06-30").await;
    assert_eq!(found[0], ("2026-03".to_string(), 130_000, false));
    assert_eq!(found[1], ("2026-04".to_string(), 130_000, false));
    assert_eq!(found[2], ("2026-05".to_string(), 90_000, false));
    assert_eq!(
        current_balance(&ana).await,
        -100_000 - 100_000 - 130_000 - 130_000 - 90_000
    );
}

#[tokio::test]
async fn the_description_and_category_can_change_and_apply_to_every_occurrence() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let rent = create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;
    let leisure = category_id(&ana, "Lazer", "expense").await;

    let response = ana
        .patch_json(
            &format!("/recurring-transactions/{rent}"),
            &json!({ "description": "  Clube ", "category_id": leisure }),
        )
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["recurring_transaction"]["description"], json!("Clube"));
    assert_eq!(
        body["recurring_transaction"]["amount_cents"],
        json!(100_000)
    );
    let listed = statement(&ana, "?to=2026-02-28").await;
    for item in listed["items"].as_array().unwrap() {
        assert_eq!(item["description"], json!("Clube"));
        assert_eq!(item["category"]["name"], json!("Lazer"));
    }

    // The Category must keep the type.
    let salary = category_id(&ana, "Salário", "income").await;
    let wrong_type = ana
        .patch_json(
            &format!("/recurring-transactions/{rent}"),
            &json!({ "category_id": salary }),
        )
        .await;
    assert_eq!(wrong_type.status(), 422);
}

/// Ends a recurring Transaction; `last_month` is optional.
async fn end(browser: &Session, id: &str, last_month: Option<&str>) -> reqwest::Response {
    match last_month {
        Some(month) => {
            browser
                .post_json(
                    &format!("/recurring-transactions/{id}/end"),
                    &json!({ "last_month": month }),
                )
                .await
        }
        None => {
            browser
                .post_empty(&format!("/recurring-transactions/{id}/end"))
                .await
        }
    }
}

#[tokio::test]
async fn ending_removes_the_future_projections_and_keeps_the_history() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let rent = create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;

    // March 2nd: January and February fell; March 5th is still a Projection.
    app.set_today(2026, 3, 2);
    let response = end(&ana, &rent, None).await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["recurring_transaction"]["ended"], json!(true));
    assert_eq!(body["recurring_transaction"]["end_month"], json!("2026-02"));
    assert_eq!(body["recurring_transaction"]["next_date"], Value::Null);

    let found = occurrences(&statement(&ana, "").await);
    assert_eq!(
        found,
        vec![
            ("2026-02-05".to_string(), 100_000, false),
            ("2026-01-05".to_string(), 100_000, false),
        ]
    );
    // Nothing more is ever charged, however far the clock goes.
    app.set_today(2026, 9, 30);
    assert_eq!(current_balance(&ana).await, -200_000);

    // It is listed as ended, and an ended one is not changed any more.
    let listed: Value = ana
        .get("/recurring-transactions")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(listed["recurring_transactions"][0]["ended"], json!(true));
    for response in [
        end(&ana, &rent, None).await,
        change_amount(&ana, &rent, 1).await,
    ] {
        assert_eq!(response.status(), 409);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], json!("recurring_transaction_ended"));
    }
}

#[tokio::test]
async fn ending_after_the_day_of_the_month_keeps_this_months_occurrence() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let rent = create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;

    app.set_today(2026, 3, 10);
    let response = end(&ana, &rent, None).await;

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["recurring_transaction"]["end_month"], json!("2026-03"));
    assert_eq!(current_balance(&ana).await, -300_000);
    let found = occurrences(&statement(&ana, "").await);
    assert_eq!(found.len(), 3);
    assert!(found.iter().all(|(_, _, projection)| !projection));
}

#[tokio::test]
async fn ending_can_pick_a_later_last_month_but_never_cut_history() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let rent = create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;
    app.set_today(2026, 3, 2); // January and February fell

    let before_history = end(&ana, &rent, Some("2026-01")).await;
    assert_eq!(before_history.status(), 422);
    let body: Value = before_history.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["last_month"],
        json!("last_month_before_history")
    );
    let malformed = end(&ana, &rent, Some("2026-6")).await;
    assert_eq!(malformed.status(), 400);

    let response = end(&ana, &rent, Some("2026-05")).await;
    assert_eq!(response.status(), 200);
    let found = occurrences(&statement(&ana, "").await);
    assert_eq!(found.len(), 5); // Jan to May
    assert_eq!(found[0], ("2026-05-05".to_string(), 100_000, true));
}

#[tokio::test]
async fn a_fixed_transaction_can_start_in_a_later_month_but_not_in_a_past_one() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    let category = category_id(&ana, "Salário", "income").await;
    let body = |start: &str| {
        json!({
            "type": "income", "amount_cents": 500_000, "day_of_month": 10,
            "description": "Salário novo", "category_id": category, "start_month": start,
        })
    };

    let past = create_with(&ana, body("2025-12")).await;
    assert_eq!(past.status(), 422);
    let error: Value = past.json().await.unwrap();
    assert_eq!(
        error["error"]["fields"]["start_month"],
        json!("start_month_past")
    );
    assert_eq!(create_with(&ana, body("dezembro")).await.status(), 400);

    let later = create_with(&ana, body("2026-03")).await;
    assert_eq!(later.status(), 201);
    let created: Value = later.json().await.unwrap();
    assert_eq!(
        created["recurring_transaction"]["start_month"],
        json!("2026-03")
    );
    assert_eq!(
        created["recurring_transaction"]["next_date"],
        json!("2026-03-10")
    );

    app.set_today(2026, 2, 28);
    assert_eq!(current_balance(&ana).await, 0);
    app.set_today(2026, 3, 10);
    assert_eq!(current_balance(&ana).await, 500_000);
}

#[tokio::test]
async fn creating_validates_the_fields_and_the_category() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let expense_category = category_id(&ana, "Moradia", "expense").await;
    let base = json!({
        "type": "expense", "amount_cents": 100_000, "day_of_month": 5,
        "description": "Aluguel", "category_id": expense_category,
    });
    let with = |field: &str, value: Value| {
        let mut body = base.clone();
        body[field] = value;
        body
    };

    for (field, value, rule) in [
        ("day_of_month", json!(0), "day_of_month_range"),
        ("day_of_month", json!(32), "day_of_month_range"),
        ("amount_cents", json!(0), "amount_range"),
        ("description", json!("   "), "description_length"),
        ("type", json!("income"), "category_type_mismatch"),
    ] {
        let response = create_with(&ana, with(field, value.clone())).await;
        assert_eq!(response.status(), 422, "{field} = {value}");
        let body: Value = response.json().await.unwrap();
        let fields = &body["error"]["fields"];
        let key = if rule.starts_with("category") {
            "category_id"
        } else {
            field
        };
        assert_eq!(fields[key], json!(rule), "{field} = {value}");
    }

    // A deactivated Category cannot take a new fixed transaction.
    let response = ana
        .post_empty(&format!("/categories/{expense_category}/deactivate"))
        .await;
    assert_eq!(response.status(), 200);
    let inactive = create_with(&ana, base.clone()).await;
    assert_eq!(inactive.status(), 422);
    let body: Value = inactive.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["category_id"],
        json!("category_inactive")
    );
}

#[tokio::test]
async fn each_user_sees_only_their_own_fixed_transactions() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let rent = create_recurring(&ana, "expense", 100_000, 5, "Aluguel").await;

    let listed: Value = ana
        .get("/recurring-transactions")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(
        listed["recurring_transactions"].as_array().unwrap().len(),
        1
    );
    let item = &listed["recurring_transactions"][0];
    assert_eq!(item["description"], json!("Aluguel"));
    assert_eq!(item["day_of_month"], json!(5));
    assert_eq!(item["start_month"], json!("2026-01"));
    assert_eq!(item["next_date"], json!("2026-02-05"));
    assert_eq!(item["ended"], json!(false));

    let theirs: Value = bia
        .get("/recurring-transactions")
        .await
        .json()
        .await
        .unwrap();
    assert!(
        theirs["recurring_transactions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(occurrences(&statement(&bia, "").await).is_empty());
    app.set_today(2026, 6, 30);
    assert_eq!(current_balance(&bia).await, 0);
    assert_eq!(change_amount(&bia, &rent, 1).await.status(), 404);
    assert_eq!(end(&bia, &rent, None).await.status(), 404);
    assert_eq!(change_amount(&bia, "not-a-uuid", 1).await.status(), 404);
}

#[tokio::test]
async fn fixed_transactions_need_a_session() {
    let app = TestApp::spawn().await;
    let anonymous = app.session();

    assert_eq!(anonymous.get("/recurring-transactions").await.status(), 401);
    assert_eq!(
        anonymous
            .delete(&occurrence_path(
                "6f1d1b0e-6a7c-4a3e-9d8e-0a1b2c3d4e5f",
                "2026-03"
            ))
            .await
            .status(),
        401
    );
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
