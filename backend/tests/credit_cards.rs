//! Credit cards (UC16): registering, editing the closing and due days, and
//! deactivating, through the HTTP API.

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

async fn create_card(browser: &Session, name: &str, closing_day: u32, due_day: u32) -> Value {
    let response = browser
        .post_json(
            "/credit-cards",
            &json!({ "name": name, "closing_day": closing_day, "due_day": due_day }),
        )
        .await;
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    body["credit_card"].clone()
}

#[tokio::test]
async fn a_user_registers_a_card_with_its_closing_and_due_days() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let card = create_card(&ana, "Nubank", 10, 17).await;

    assert_eq!(card["name"], json!("Nubank"));
    assert_eq!(card["closing_day"], json!(10));
    assert_eq!(card["due_day"], json!(17));
    assert_eq!(card["active"], json!(true));
    let list: Value = ana.get("/credit-cards").await.json().await.unwrap();
    assert_eq!(list["credit_cards"].as_array().unwrap().len(), 1);
    assert_eq!(list["credit_cards"][0]["id"], card["id"]);
}

#[tokio::test]
async fn a_card_needs_a_name_and_days_from_1_to_31() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let response = ana
        .post_json(
            "/credit-cards",
            &json!({ "name": "  ", "closing_day": 0, "due_day": 32 }),
        )
        .await;

    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("validation_failed"));
    assert_eq!(body["error"]["fields"]["name"], json!("name_length"));
    assert_eq!(body["error"]["fields"]["closing_day"], json!("day_range"));
    assert_eq!(body["error"]["fields"]["due_day"], json!("day_range"));
}

#[tokio::test]
async fn a_card_name_is_unique_per_user_ignoring_case() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    create_card(&ana, "Nubank", 10, 17).await;

    let again = ana
        .post_json(
            "/credit-cards",
            &json!({ "name": "nubank", "closing_day": 1, "due_day": 8 }),
        )
        .await;
    assert_eq!(again.status(), 409);
    let body: Value = again.json().await.unwrap();
    assert_eq!(body["error"]["code"], json!("credit_card_name_taken"));

    // Another user may use the same name.
    create_card(&bia, "Nubank", 10, 17).await;
}

#[tokio::test]
async fn a_user_only_sees_and_edits_their_own_cards() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let card = create_card(&ana, "Nubank", 10, 17).await;
    let id = card["id"].as_str().unwrap();

    let list: Value = bia.get("/credit-cards").await.json().await.unwrap();
    assert_eq!(list["credit_cards"].as_array().unwrap().len(), 0);
    let edit = bia
        .patch_json(&format!("/credit-cards/{id}"), &json!({ "name": "x" }))
        .await;
    assert_eq!(edit.status(), 404);
    let deactivate = bia
        .post_empty(&format!("/credit-cards/{id}/deactivate"))
        .await;
    assert_eq!(deactivate.status(), 404);
}

#[tokio::test]
async fn a_card_without_invoices_can_be_deactivated_and_reactivated() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let card = create_card(&ana, "Nubank", 10, 17).await;
    let id = card["id"].as_str().unwrap();

    let off = ana
        .post_empty(&format!("/credit-cards/{id}/deactivate"))
        .await;
    assert_eq!(off.status(), 200);
    let body: Value = off.json().await.unwrap();
    assert_eq!(body["credit_card"]["active"], json!(false));
    // The card stays in the list (history), marked as inactive.
    let list: Value = ana.get("/credit-cards").await.json().await.unwrap();
    assert_eq!(list["credit_cards"][0]["active"], json!(false));

    let on = ana
        .post_empty(&format!("/credit-cards/{id}/reactivate"))
        .await;
    assert_eq!(on.status(), 200);
    let body: Value = on.json().await.unwrap();
    assert_eq!(body["credit_card"]["active"], json!(true));
}
