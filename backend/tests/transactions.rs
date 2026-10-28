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

/// Creates an expense in "Alimentação" and returns the transaction JSON.
async fn create_expense(browser: &Session, date: &str, description: &str) -> Value {
    let response = create(
        browser,
        "expense",
        1000,
        date,
        description,
        ("Alimentação", "expense"),
    )
    .await;
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    body["transaction"].clone()
}

fn id_of(transaction: &Value) -> String {
    transaction["id"].as_str().unwrap().to_string()
}

async fn edit(browser: &Session, id: &str, body: Value) -> reqwest::Response {
    browser
        .patch_json(&format!("/transactions/{id}"), &body)
        .await
}

#[tokio::test]
async fn a_date_after_today_is_a_projection_until_the_day_arrives() {
    let app = TestApp::spawn().await; // today is 2026-01-15
    let ana = signed_up(&app, "ana@example.com").await;

    let today = create_expense(&ana, "2026-01-15", "Hoje").await;
    let tomorrow = create_expense(&ana, "2026-01-16", "Amanhã").await;
    let yesterday = create_expense(&ana, "2026-01-14", "Ontem").await;

    assert_eq!(today["is_projection"], json!(false));
    assert_eq!(tomorrow["is_projection"], json!(true));
    assert_eq!(yesterday["is_projection"], json!(false));
    // The Projection is in the list, marked as such.
    let items = list(&ana, "").await["items"].as_array().unwrap().clone();
    let marked: Vec<(&str, bool)> = items
        .iter()
        .map(|item| {
            (
                item["description"].as_str().unwrap(),
                item["is_projection"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        marked,
        [("Amanhã", true), ("Hoje", false), ("Ontem", false)]
    );

    // When the day arrives it stops being a Projection: nothing was stored.
    app.set_today(2026, 1, 16);
    let items = list(&ana, "").await["items"].as_array().unwrap().clone();
    assert!(
        items
            .iter()
            .all(|item| item["is_projection"] == json!(false))
    );
}

#[tokio::test]
async fn editing_changes_amount_date_description_and_category_but_not_the_type() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let original = create_expense(&ana, "2026-01-10", "Almoço").await;
    let transport = category_id(&ana, "Transporte", "expense").await;

    let response = edit(
        &ana,
        &id_of(&original),
        json!({
            "amount_cents": 2599,
            "date": "2026-02-01",
            "description": "  Táxi  ",
            "category_id": transport,
            // The type cannot be changed: this is ignored.
            "type": "income",
        }),
    )
    .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    let edited = &body["transaction"];
    assert_eq!(edited["id"], original["id"]);
    assert_eq!(edited["type"], json!("expense"));
    assert_eq!(edited["amount_cents"], json!(2599));
    assert_eq!(edited["date"], json!("2026-02-01"));
    assert_eq!(edited["description"], json!("Táxi"));
    assert_eq!(edited["category"]["name"], json!("Transporte"));
    assert_eq!(edited["is_projection"], json!(true));
    let listed = list(&ana, "").await;
    assert_eq!(listed["total"], json!(1));
    assert_eq!(listed["items"][0]["description"], json!("Táxi"));
}

#[tokio::test]
async fn editing_only_what_is_sent_leaves_the_rest_untouched() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let original = create_expense(&ana, "2026-01-10", "Almoço").await;

    let response = edit(&ana, &id_of(&original), json!({ "description": "Jantar" })).await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    let edited = &body["transaction"];
    assert_eq!(edited["description"], json!("Jantar"));
    assert_eq!(edited["amount_cents"], original["amount_cents"]);
    assert_eq!(edited["date"], original["date"]);
    assert_eq!(edited["category"], original["category"]);
}

#[tokio::test]
async fn editing_follows_the_same_rules_as_creating() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let id = id_of(&create_expense(&ana, "2026-01-10", "Almoço").await);
    let salary = category_id(&ana, "Salário", "income").await;
    let leisure = category_id(&ana, "Lazer", "expense").await;
    ana.post_empty(&format!("/categories/{leisure}/deactivate"))
        .await;

    expect_field_error(
        edit(&ana, &id, json!({ "amount_cents": 0 })).await,
        "amount_cents",
        "amount_range",
    )
    .await;
    expect_field_error(
        edit(&ana, &id, json!({ "category_id": salary })).await,
        "category_id",
        "category_type_mismatch",
    )
    .await;
    expect_field_error(
        edit(&ana, &id, json!({ "category_id": leisure })).await,
        "category_id",
        "category_inactive",
    )
    .await;
    expect_field_error(
        edit(&ana, &id, json!({ "description": " " })).await,
        "description",
        "description_length",
    )
    .await;
    assert_eq!(
        edit(&ana, &id, json!({ "date": "2026-13-01" }))
            .await
            .status(),
        400
    );
    // Nothing changed.
    let listed = list(&ana, "").await;
    assert_eq!(listed["items"][0]["amount_cents"], json!(1000));
    assert_eq!(listed["items"][0]["category"]["name"], json!("Alimentação"));
}

#[tokio::test]
async fn a_transaction_keeps_its_category_after_it_is_deactivated_and_can_still_be_edited() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let old = id_of(&create_expense(&ana, "2026-01-10", "Almoço").await);
    let food = category_id(&ana, "Alimentação", "expense").await;
    ana.post_empty(&format!("/categories/{food}/deactivate"))
        .await;

    let listed = list(&ana, "").await;
    assert_eq!(listed["items"][0]["category"]["name"], json!("Alimentação"));
    assert_eq!(listed["items"][0]["category"]["active"], json!(false));
    // Editing without touching the category (or sending the same one) is fine.
    assert_eq!(
        edit(&ana, &old, json!({ "description": "Almoço fora" }))
            .await
            .status(),
        200
    );
    assert_eq!(
        edit(
            &ana,
            &old,
            json!({ "amount_cents": 1200, "category_id": food })
        )
        .await
        .status(),
        200
    );
}

#[tokio::test]
async fn editing_or_deleting_an_unknown_transaction_is_not_found() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    for id in ["00000000-0000-4000-8000-000000000000", "not-an-id"] {
        let response = edit(&ana, id, json!({ "description": "x" })).await;
        assert_eq!(response.status(), 404, "edit {id}");
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], json!("transaction_not_found"));
        assert_eq!(
            ana.delete(&format!("/transactions/{id}")).await.status(),
            404,
            "delete {id}"
        );
    }
}

#[tokio::test]
async fn deleting_removes_the_transaction() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let keep = create_expense(&ana, "2026-01-10", "Fica").await;
    let gone = create_expense(&ana, "2026-01-11", "Some").await;

    let response = ana.delete(&format!("/transactions/{}", id_of(&gone))).await;

    assert_eq!(response.status(), 204);
    let listed = list(&ana, "").await;
    assert_eq!(listed["total"], json!(1));
    assert_eq!(listed["items"][0]["id"], keep["id"]);
    // Deleting twice: the second one finds nothing.
    let again = ana.delete(&format!("/transactions/{}", id_of(&gone))).await;
    assert_eq!(again.status(), 404);
}

#[tokio::test]
async fn users_cannot_see_edit_or_delete_each_others_transactions() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    let anas = create_expense(&ana, "2026-01-10", "Da Ana").await;
    create_expense(&bia, "2026-01-12", "Da Bia").await;
    let id = id_of(&anas);

    assert_eq!(
        edit(&bia, &id, json!({ "description": "Invadido" }))
            .await
            .status(),
        404
    );
    assert_eq!(
        bia.delete(&format!("/transactions/{id}")).await.status(),
        404
    );

    let bias = list(&bia, "").await;
    assert_eq!(bias["total"], json!(1));
    assert_eq!(bias["items"][0]["description"], json!("Da Bia"));
    let anas_list = list(&ana, "").await;
    assert_eq!(anas_list["total"], json!(1));
    assert_eq!(anas_list["items"][0]["description"], json!("Da Ana"));
}

#[tokio::test]
async fn transactions_need_a_session() {
    let app = TestApp::spawn().await;
    let nobody = app.session();
    let id = "00000000-0000-4000-8000-000000000000";

    assert_eq!(nobody.get("/transactions").await.status(), 401);
    assert_eq!(create_with(&nobody, valid_body(id)).await.status(), 401);
    assert_eq!(edit(&nobody, id, json!({})).await.status(), 401);
    assert_eq!(
        nobody.delete(&format!("/transactions/{id}")).await.status(),
        401
    );
}
