//! Income and expense Transactions outside the credit card (UC04, UC06, UC07
//! and UC08), through the HTTP API.

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
    let response = browser.get("/categories").await;
    let body: Value = response.json().await.unwrap();
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

/// Posts a transaction; `category` is `(name, type)` of a default category.
async fn create(
    browser: &Session,
    kind: &str,
    amount_cents: i64,
    date: &str,
    description: &str,
    category: (&str, &str),
) -> reqwest::Response {
    let category_id = category_id(browser, category.0, category.1).await;
    browser
        .post_json(
            "/transactions",
            &json!({
                "type": kind,
                "amount_cents": amount_cents,
                "date": date,
                "description": description,
                "category_id": category_id,
            }),
        )
        .await
}

async fn list(browser: &Session, query: &str) -> Value {
    let response = browser.get(&format!("/transactions{query}")).await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

#[tokio::test]
async fn an_expense_is_created_and_appears_in_the_list_with_its_category() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let response = create(
        &ana,
        "expense",
        4590,
        "2026-01-10",
        "  Supermercado  ",
        ("Alimentação", "expense"),
    )
    .await;

    assert_eq!(response.status(), 201);
    let created: Value = response.json().await.unwrap();
    let transaction = &created["transaction"];
    assert_eq!(transaction["type"], json!("expense"));
    assert_eq!(transaction["amount_cents"], json!(4590));
    assert_eq!(transaction["date"], json!("2026-01-10"));
    assert_eq!(transaction["description"], json!("Supermercado"));
    assert_eq!(transaction["category"]["name"], json!("Alimentação"));
    assert_eq!(transaction["category"]["type"], json!("expense"));
    assert_eq!(transaction["is_projection"], json!(false));

    let listed = list(&ana, "").await;
    let items = listed["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["kind"], json!("transaction"));
    assert_eq!(items[0]["id"], transaction["id"]);
    assert_eq!(listed["total"], json!(1));
}

/// Posts a transaction whose category is given by id.
async fn create_with(browser: &Session, body: Value) -> reqwest::Response {
    browser.post_json("/transactions", &body).await
}

async fn expect_field_error(response: reqwest::Response, field: &str, rule: &str) {
    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("validation_failed"));
    assert_eq!(body["error"]["fields"][field], json!(rule));
}

fn valid_body(category_id: &str) -> Value {
    json!({
        "type": "expense",
        "amount_cents": 1000,
        "date": "2026-01-10",
        "description": "Almoço",
        "category_id": category_id,
    })
}

#[tokio::test]
async fn the_amount_must_be_greater_than_zero_and_within_the_limit() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let food = category_id(&ana, "Alimentação", "expense").await;

    for amount in [0, -500, 100_000_000_001_i64] {
        let mut body = valid_body(&food);
        body["amount_cents"] = json!(amount);
        expect_field_error(
            create_with(&ana, body).await,
            "amount_cents",
            "amount_range",
        )
        .await;
    }
    // Reais with decimals are not cents: the API only takes whole cents.
    let mut body = valid_body(&food);
    body["amount_cents"] = json!(10.5);
    assert_eq!(create_with(&ana, body).await.status(), 400);

    assert_eq!(list(&ana, "").await["total"], json!(0));
}

#[tokio::test]
async fn the_description_is_required_and_the_date_must_be_a_calendar_day() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let food = category_id(&ana, "Alimentação", "expense").await;

    let mut blank = valid_body(&food);
    blank["description"] = json!("   ");
    expect_field_error(
        create_with(&ana, blank).await,
        "description",
        "description_length",
    )
    .await;

    // A time of day or a made-up day is not a calendar day.
    for date in ["2026-01-10T12:00:00Z", "2026-02-30", "10/01/2026"] {
        let mut body = valid_body(&food);
        body["date"] = json!(date);
        assert_eq!(create_with(&ana, body).await.status(), 400, "date {date}");
    }
}

#[tokio::test]
async fn the_category_must_have_the_same_type_as_the_transaction() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let salary = category_id(&ana, "Salário", "income").await;

    // An expense cannot use an income category.
    let response = create_with(&ana, valid_body(&salary)).await;

    expect_field_error(response, "category_id", "category_type_mismatch").await;
    assert_eq!(list(&ana, "").await["total"], json!(0));
}

#[tokio::test]
async fn a_deactivated_category_is_refused_for_a_new_transaction() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let leisure = category_id(&ana, "Lazer", "expense").await;
    ana.post_empty(&format!("/categories/{leisure}/deactivate"))
        .await;

    let response = create_with(&ana, valid_body(&leisure)).await;

    expect_field_error(response, "category_id", "category_inactive").await;
}

#[tokio::test]
async fn another_users_category_or_an_unknown_one_is_refused() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let anas_food = category_id(&ana, "Alimentação", "expense").await;

    let response = create_with(&bia, valid_body(&anas_food)).await;
    expect_field_error(response, "category_id", "category_not_found").await;

    let unknown = "00000000-0000-4000-8000-000000000000";
    let response = create_with(&bia, valid_body(unknown)).await;
    expect_field_error(response, "category_id", "category_not_found").await;
}
