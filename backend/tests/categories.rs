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
