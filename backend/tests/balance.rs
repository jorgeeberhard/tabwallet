//! The Current Balance and the Opening Balance (UC10, partial, and UC22),
//! through the HTTP API.

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

/// Records a transaction in a default category of the same type.
async fn record(browser: &Session, kind: &str, amount_cents: i64, date: &str) {
    let name = if kind == "income" {
        "Salário"
    } else {
        "Outros"
    };
    let categories: Value = browser.get("/categories").await.json().await.unwrap();
    let category_id = categories["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|category| category["name"] == json!(name) && category["type"] == json!(kind))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let response = browser
        .post_json(
            "/transactions",
            &json!({
                "type": kind,
                "amount_cents": amount_cents,
                "date": date,
                "description": "test",
                "category_id": category_id,
            }),
        )
        .await;
    assert_eq!(response.status(), 201);
}

async fn balance(browser: &Session) -> Value {
    let response = browser.get("/balance").await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

#[tokio::test]
async fn a_new_user_has_a_zero_balance_and_no_opening_balance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let body = balance(&ana).await;

    assert_eq!(body["current_balance_cents"], json!(0));
    assert_eq!(body["opening_balance_set"], json!(false));
    assert_eq!(body["opening_balance_cents"], Value::Null);
}

#[tokio::test]
async fn the_balance_is_incomes_minus_expenses_up_to_today() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    record(&ana, "income", 500_000, "2026-01-05").await;
    record(&ana, "expense", 120_050, "2026-01-10").await;
    record(&ana, "expense", 30_000, "2026-01-15").await; // today counts

    assert_eq!(balance(&ana).await["current_balance_cents"], json!(349_950));
}

#[tokio::test]
async fn a_projection_enters_the_balance_when_its_date_arrives() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;
    record(&ana, "income", 100_000, "2026-01-10").await;
    record(&ana, "expense", 40_000, "2026-01-20").await; // a Projection

    assert_eq!(balance(&ana).await["current_balance_cents"], json!(100_000));

    app.set_today(2026, 1, 19);
    assert_eq!(balance(&ana).await["current_balance_cents"], json!(100_000));

    app.set_today(2026, 1, 20);
    assert_eq!(balance(&ana).await["current_balance_cents"], json!(60_000));
}

#[tokio::test]
async fn the_balance_can_go_negative_and_nothing_blocks_a_transaction() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    record(&ana, "income", 10_000, "2026-01-01").await;

    record(&ana, "expense", 25_000, "2026-01-02").await;

    assert_eq!(balance(&ana).await["current_balance_cents"], json!(-15_000));
}

#[tokio::test]
async fn each_user_sees_only_their_own_balance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    record(&ana, "income", 70_000, "2026-01-01").await;

    assert_eq!(balance(&ana).await["current_balance_cents"], json!(70_000));
    assert_eq!(balance(&bia).await["current_balance_cents"], json!(0));
}

#[tokio::test]
async fn the_balance_needs_a_session() {
    let app = TestApp::spawn().await;

    let response = app.session().get("/balance").await;

    assert_eq!(response.status(), 401);
}

async fn set_opening(browser: &Session, amount_cents: i64) -> reqwest::Response {
    browser
        .post_json("/balance/opening", &json!({ "amount_cents": amount_cents }))
        .await
}

#[tokio::test]
async fn the_opening_balance_is_informed_once_and_counts_in_the_balance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    record(&ana, "expense", 20_000, "2026-01-05").await;

    let response = set_opening(&ana, 150_000).await;

    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["current_balance_cents"], json!(130_000));
    assert_eq!(body["opening_balance_set"], json!(true));
    assert_eq!(body["opening_balance_cents"], json!(150_000));
    assert_eq!(balance(&ana).await, body);
}

#[tokio::test]
async fn a_second_opening_balance_is_rejected_and_the_first_stays() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    assert_eq!(set_opening(&ana, 150_000).await.status(), 201);

    let response = set_opening(&ana, 999_999).await;

    assert_eq!(response.status(), 409);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("opening_balance_already_set"));
    let after = balance(&ana).await;
    assert_eq!(after["opening_balance_cents"], json!(150_000));
    assert_eq!(after["current_balance_cents"], json!(150_000));
}

#[tokio::test]
async fn the_opening_balance_may_be_negative() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    record(&ana, "income", 10_000, "2026-01-05").await;

    let response = set_opening(&ana, -45_000).await;

    assert_eq!(response.status(), 201);
    let body = balance(&ana).await;
    assert_eq!(body["opening_balance_cents"], json!(-45_000));
    assert_eq!(body["current_balance_cents"], json!(-35_000));
}

#[tokio::test]
async fn a_zero_opening_balance_counts_as_informed() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    assert_eq!(set_opening(&ana, 0).await.status(), 201);

    let body = balance(&ana).await;
    assert_eq!(body["opening_balance_set"], json!(true));
    assert_eq!(body["opening_balance_cents"], json!(0));
    assert_eq!(set_opening(&ana, 100).await.status(), 409);
}

#[tokio::test]
async fn an_absurd_or_malformed_opening_balance_is_refused_and_does_not_use_up_the_one_chance() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let too_big = set_opening(&ana, 100_000_000_001).await;
    assert_eq!(too_big.status(), 422);
    let body: Value = too_big.json().await.unwrap();
    assert_eq!(
        body["error"]["fields"]["amount_cents"],
        json!("amount_range")
    );
    assert_eq!(set_opening(&ana, -100_000_000_001).await.status(), 422);
    let not_integer = ana
        .post_json("/balance/opening", &json!({ "amount_cents": 10.5 }))
        .await;
    assert_eq!(not_integer.status(), 400);
    let missing = ana.post_json("/balance/opening", &json!({})).await;
    assert_eq!(missing.status(), 400);

    assert_eq!(balance(&ana).await["opening_balance_set"], json!(false));
    assert_eq!(set_opening(&ana, 100_000_000_000).await.status(), 201);
}

#[tokio::test]
async fn the_opening_balance_belongs_to_one_user() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    assert_eq!(set_opening(&ana, 150_000).await.status(), 201);

    assert_eq!(balance(&bia).await["opening_balance_set"], json!(false));
    assert_eq!(set_opening(&bia, 7_000).await.status(), 201);
    assert_eq!(balance(&ana).await["opening_balance_cents"], json!(150_000));
}

#[tokio::test]
async fn setting_the_opening_balance_needs_a_session() {
    let app = TestApp::spawn().await;

    let response = set_opening(&app.session(), 100).await;

    assert_eq!(response.status(), 401);
}
