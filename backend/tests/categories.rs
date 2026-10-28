//! Categories (UC09): default categories, create, rename, deactivate and
//! reactivate, through the HTTP API.

mod common;

use common::{Session, TestApp};
use serde_json::{Value, json};

const PASSWORD: &str = "correct horse battery";

const DEFAULT_EXPENSES: [&str; 9] = [
    "Alimentação",
    "Moradia",
    "Transporte",
    "Saúde",
    "Educação",
    "Lazer",
    "Assinaturas",
    "Juros e encargos",
    "Outros",
];
const DEFAULT_INCOMES: [&str; 2] = ["Salário", "Outros"];

async fn signed_up(app: &TestApp, email: &str) -> Session {
    let browser = app.session();
    let response = browser.sign_up(email, PASSWORD).await;
    assert_eq!(response.status(), 201);
    browser
}

async fn categories(browser: &Session, query: &str) -> Vec<Value> {
    let response = browser.get(&format!("/categories{query}")).await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    body["categories"].as_array().unwrap().clone()
}

fn names_of_type(categories: &[Value], kind: &str) -> Vec<String> {
    let mut names: Vec<String> = categories
        .iter()
        .filter(|category| category["type"] == json!(kind))
        .map(|category| category["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

fn sorted(names: &[&str]) -> Vec<String> {
    let mut names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    names.sort();
    names
}

#[tokio::test]
async fn a_new_user_receives_the_default_expense_and_income_categories_all_active() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let all = categories(&ana, "").await;

    assert_eq!(names_of_type(&all, "expense"), sorted(&DEFAULT_EXPENSES));
    assert_eq!(names_of_type(&all, "income"), sorted(&DEFAULT_INCOMES));
    assert!(all.iter().all(|category| category["active"] == json!(true)));
}

#[tokio::test]
async fn categories_need_a_session() {
    let app = TestApp::spawn().await;

    let response = app.session().get("/categories").await;

    assert_eq!(response.status(), 401);
}

async fn create(browser: &Session, name: &str, kind: &str) -> reqwest::Response {
    browser
        .post_json("/categories", &json!({ "name": name, "type": kind }))
        .await
}

async fn error_code(response: reqwest::Response) -> (Value, String) {
    let body: Value = response.json().await.unwrap();
    let code = body["error"]["code"].as_str().unwrap().to_string();
    (body, code)
}

#[tokio::test]
async fn creating_a_category_adds_it_active_to_the_list_with_its_type() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let response = create(&ana, "  Pets  ", "expense").await;

    assert_eq!(response.status(), 201);
    let created: Value = response.json().await.unwrap();
    assert_eq!(created["category"]["name"], json!("Pets"));
    assert_eq!(created["category"]["type"], json!("expense"));
    assert_eq!(created["category"]["active"], json!(true));
    let expenses = categories(&ana, "?type=expense").await;
    assert!(names_of_type(&expenses, "expense").contains(&"Pets".to_string()));
    let incomes = categories(&ana, "?type=income").await;
    assert!(!names_of_type(&incomes, "income").contains(&"Pets".to_string()));
    assert!(names_of_type(&incomes, "expense").is_empty());
}

#[tokio::test]
async fn creating_rejects_a_blank_name_and_an_unknown_type() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let blank = create(&ana, "   ", "expense").await;
    assert_eq!(blank.status(), 422);
    let (body, code) = error_code(blank).await;
    assert_eq!(code, "validation_failed");
    assert_eq!(body["error"]["fields"]["name"], json!("name_length"));

    let unknown_type = create(&ana, "Pets", "transfer").await;
    assert_eq!(unknown_type.status(), 400);
}

#[tokio::test]
async fn an_active_category_blocks_the_same_name_in_the_same_type_whatever_the_letter_case() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;

    let response = create(&ana, "lazer", "expense").await;

    assert_eq!(response.status(), 409);
    assert_eq!(error_code(response).await.1, "category_name_taken");
}

#[tokio::test]
async fn the_same_name_is_allowed_in_the_other_type_and_for_another_user() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;

    // "Lazer" is a default expense category: it is free as an income category.
    assert_eq!(create(&ana, "Lazer", "income").await.status(), 201);
    // Ana's new "Pets" does not matter to Bia.
    assert_eq!(create(&ana, "Pets", "expense").await.status(), 201);
    assert_eq!(create(&bia, "Pets", "expense").await.status(), 201);
}

/// The id of the user's category of this type with this name.
async fn id_of(browser: &Session, name: &str, kind: &str) -> String {
    categories(browser, "")
        .await
        .iter()
        .find(|category| category["name"] == json!(name) && category["type"] == json!(kind))
        .unwrap_or_else(|| panic!("no {kind} category named {name}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn rename(browser: &Session, id: &str, name: &str) -> reqwest::Response {
    browser
        .patch_json(&format!("/categories/{id}"), &json!({ "name": name }))
        .await
}

async fn deactivate(browser: &Session, id: &str) -> reqwest::Response {
    browser
        .post_empty(&format!("/categories/{id}/deactivate"))
        .await
}

async fn reactivate(browser: &Session, id: &str) -> reqwest::Response {
    browser
        .post_empty(&format!("/categories/{id}/reactivate"))
        .await
}

#[tokio::test]
async fn renaming_changes_the_name_and_keeps_the_category() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let id = id_of(&ana, "Lazer", "expense").await;

    let response = rename(&ana, &id, "Diversão").await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["category"]["id"], json!(id));
    assert_eq!(body["category"]["name"], json!("Diversão"));
    assert_eq!(id_of(&ana, "Diversão", "expense").await, id);
    // Only the letter case changes: still the same category, not a clash.
    assert_eq!(rename(&ana, &id, "diversão").await.status(), 200);
}

#[tokio::test]
async fn renaming_to_a_name_already_used_in_the_type_is_refused() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let id = id_of(&ana, "Lazer", "expense").await;

    let response = rename(&ana, &id, "moradia").await;

    assert_eq!(response.status(), 409);
    assert_eq!(error_code(response).await.1, "category_name_taken");
    assert_eq!(rename(&ana, &id, "  ").await.status(), 422);
}

#[tokio::test]
async fn renaming_to_the_name_of_a_deactivated_category_points_to_it() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let lazer = id_of(&ana, "Lazer", "expense").await;
    let moradia = id_of(&ana, "Moradia", "expense").await;
    assert_eq!(deactivate(&ana, &moradia).await.status(), 200);

    let response = rename(&ana, &lazer, "Moradia").await;

    assert_eq!(response.status(), 409);
    let (body, code) = error_code(response).await;
    assert_eq!(code, "category_inactive_exists");
    assert_eq!(body["error"]["category_id"], json!(moradia));
}

#[tokio::test]
async fn deactivating_hides_the_category_from_active_listings_but_keeps_it_and_its_name() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let id = id_of(&ana, "Lazer", "expense").await;

    let response = deactivate(&ana, &id).await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["category"]["active"], json!(false));
    let active = categories(&ana, "?type=expense&active=true").await;
    assert!(!names_of_type(&active, "expense").contains(&"Lazer".to_string()));
    let inactive = categories(&ana, "?active=false").await;
    assert_eq!(names_of_type(&inactive, "expense"), vec!["Lazer"]);
    // Deactivating again changes nothing.
    assert_eq!(deactivate(&ana, &id).await.status(), 200);
}

#[tokio::test]
async fn creating_with_the_name_of_a_deactivated_category_offers_to_reactivate_it() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let id = id_of(&ana, "Lazer", "expense").await;
    deactivate(&ana, &id).await;

    let response = create(&ana, "LAZER", "expense").await;

    assert_eq!(response.status(), 409);
    let (body, code) = error_code(response).await;
    assert_eq!(code, "category_inactive_exists");
    assert_eq!(body["error"]["category_id"], json!(id));
    // Following the offer brings the very same category back.
    let reactivated = reactivate(&ana, &id).await;
    assert_eq!(reactivated.status(), 200);
    let body: Value = reactivated.json().await.unwrap();
    assert_eq!(body["category"]["active"], json!(true));
    let active = categories(&ana, "?active=true").await;
    assert!(names_of_type(&active, "expense").contains(&"Lazer".to_string()));
}

#[tokio::test]
async fn juros_e_encargos_is_flagged_survives_a_rename_and_can_be_deactivated_without_charges() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let id = id_of(&ana, "Juros e encargos", "expense").await;

    // The flag, not the name, marks the category where Charges go.
    let renamed: Value = rename(&ana, &id, "Tarifas").await.json().await.unwrap();
    assert_eq!(renamed["category"]["is_charges"], json!(true));
    let lazer = id_of(&ana, "Lazer", "expense").await;
    let lazer: Value = rename(&ana, &lazer, "Diversão").await.json().await.unwrap();
    assert_eq!(lazer["category"]["is_charges"], json!(false));
    // No Charges exist yet, so nothing stops the user from deactivating it.
    assert_eq!(deactivate(&ana, &id).await.status(), 200);
}

#[tokio::test]
async fn nobody_can_see_or_change_the_categories_of_another_user() {
    let app = TestApp::spawn().await;
    let ana = signed_up(&app, "ana@example.com").await;
    let bia = signed_up(&app, "bia@example.com").await;
    create(&ana, "Pets", "expense").await;
    let pets = id_of(&ana, "Pets", "expense").await;

    assert!(!names_of_type(&categories(&bia, "").await, "expense").contains(&"Pets".to_string()));
    for response in [
        rename(&bia, &pets, "Bichos").await,
        deactivate(&bia, &pets).await,
        reactivate(&bia, &pets).await,
    ] {
        assert_eq!(response.status(), 404);
        assert_eq!(error_code(response).await.1, "category_not_found");
    }
    // Nothing changed for Ana.
    assert_eq!(id_of(&ana, "Pets", "expense").await, pets);
    let unknown = "00000000-0000-0000-0000-000000000000";
    assert_eq!(rename(&ana, unknown, "X").await.status(), 404);
    assert_eq!(rename(&ana, "not-a-uuid", "X").await.status(), 404);
}

#[tokio::test]
async fn a_user_who_signed_up_before_categories_existed_gets_the_defaults_from_the_migration() {
    let pool = TestApp::empty_database().await;
    // The schema as it was before this feature, plus one existing user.
    sqlx::raw_sql(include_str!("../migrations/0001_enable_citext.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0002_create_users.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let password_hash = tabwallet_backend::auth::password::hash(PASSWORD.to_string())
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind("velho@example.com")
        .bind(password_hash)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::raw_sql(include_str!("../migrations/0003_create_categories.sql"))
        .execute(&pool)
        .await
        .unwrap();

    let app = TestApp::spawn_on(pool).await;
    let velho = app.session();
    assert_eq!(
        velho.log_in("velho@example.com", PASSWORD).await.status(),
        200
    );
    let all = categories(&velho, "").await;
    assert_eq!(names_of_type(&all, "expense"), sorted(&DEFAULT_EXPENSES));
    assert_eq!(names_of_type(&all, "income"), sorted(&DEFAULT_INCOMES));
}

#[tokio::test]
async fn changing_categories_needs_a_session() {
    let app = TestApp::spawn().await;
    let visitor = app.session();
    let id = "00000000-0000-0000-0000-000000000000";

    assert_eq!(create(&visitor, "Pets", "expense").await.status(), 401);
    assert_eq!(rename(&visitor, id, "X").await.status(), 401);
    assert_eq!(deactivate(&visitor, id).await.status(), 401);
    assert_eq!(reactivate(&visitor, id).await.status(), 401);
}
