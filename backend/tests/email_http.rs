//! The HTTP e-mail sender, against a stub provider on a local port.

use std::sync::{Arc, Mutex};

use axum::{Json, Router, extract::State, http::HeaderMap, http::StatusCode, routing::post};
use serde_json::{Value, json};
use tabwallet_backend::email::{EmailMessage, EmailSender, HttpEmailSender};
use tokio::net::TcpListener;

type Captured = Arc<Mutex<Vec<(String, Value)>>>;

/// Starts a provider stub that records the Authorization header and body of
/// each request and answers with `status`. Returns its URL and the records.
async fn stub_provider(status: StatusCode) -> (String, Captured) {
    let captured: Captured = Arc::default();
    let app = Router::new()
        .route(
            "/emails",
            post(
                move |State(captured): State<Captured>,
                      headers: HeaderMap,
                      Json(body): Json<Value>| async move {
                    let auth = headers["authorization"].to_str().unwrap().to_string();
                    captured.lock().unwrap().push((auth, body));
                    (status, "stub answer")
                },
            ),
        )
        .with_state(captured.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/emails", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, captured)
}

fn message() -> EmailMessage {
    EmailMessage {
        to: "ana@example.com".into(),
        subject: "Assunto".into(),
        text: "Corpo".into(),
    }
}

#[tokio::test]
async fn posts_the_message_to_the_provider_with_the_api_key() {
    let (url, captured) = stub_provider(StatusCode::OK).await;
    let sender =
        HttpEmailSender::new(&url, "secret-key", "TabWallet <no-reply@tabwallet.app>").unwrap();

    sender.send(message()).await.unwrap();

    let captured = captured.lock().unwrap();
    assert_eq!(captured.len(), 1);
    let (auth, body) = &captured[0];
    assert_eq!(auth, "Bearer secret-key");
    assert_eq!(
        *body,
        json!({
            "from": "TabWallet <no-reply@tabwallet.app>",
            "to": ["ana@example.com"],
            "subject": "Assunto",
            "text": "Corpo",
        })
    );
}

#[tokio::test]
async fn reports_an_error_when_the_provider_refuses() {
    let (url, _) = stub_provider(StatusCode::UNPROCESSABLE_ENTITY).await;
    let sender = HttpEmailSender::new(&url, "secret-key", "no-reply@tabwallet.app").unwrap();

    let error = sender.send(message()).await.unwrap_err();

    assert!(error.0.contains("422"), "{}", error.0);
}
