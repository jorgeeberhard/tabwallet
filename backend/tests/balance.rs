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
