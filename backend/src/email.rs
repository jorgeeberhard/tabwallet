//! Transactional e-mail (today only the password-reset link).
//!
//! Use cases depend on the [`EmailSender`] trait. `main` picks the
//! implementation from environment variables ([`EmailSettings::from_env`]);
//! integration tests plug in a recording fake instead.
//!
//! - [`LogEmailSender`] writes the message to the log instead of sending it.
//!   It exists for local development, where there is no mail provider, and is
//!   refused unless `APP_ENV=development`, because the log would then hold
//!   working reset links.
//! - [`HttpEmailSender`] posts the message to an HTTP e-mail API (the JSON
//!   format of Resend: `from`, `to`, `subject`, `text`, with a Bearer key).

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::json;

/// A plain-text e-mail to one recipient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
#[error("could not send the e-mail: {0}")]
pub struct EmailError(pub String);

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError>;
}

/// Development sender: logs the message (and so the reset link) at info level.
pub struct LogEmailSender;

#[async_trait]
impl EmailSender for LogEmailSender {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError> {
        tracing::info!(
            "e-mail not sent (no provider configured, development only)\nto: {}\nsubject: {}\n\n{}",
            message.to,
            message.subject,
            message.text
        );
        Ok(())
    }
}

/// Sends through an HTTP e-mail API.
pub struct HttpEmailSender {
    client: reqwest::Client,
    api_url: String,
    api_key: String,
    from: String,
}

const DEFAULT_API_URL: &str = "https://api.resend.com/emails";

impl HttpEmailSender {
    pub fn new(api_url: &str, api_key: &str, from: &str) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| format!("could not build the HTTP client: {error}"))?;
        Ok(Self {
            client,
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            from: from.to_string(),
        })
    }
}

#[async_trait]
impl EmailSender for HttpEmailSender {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError> {
        let response = self
            .client
            .post(&self.api_url)
            .bearer_auth(&self.api_key)
            .json(&json!({
                "from": self.from,
                "to": [message.to],
                "subject": message.subject,
                "text": message.text,
            }))
            .send()
            .await
            .map_err(|error| EmailError(error.to_string()))?;
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            // The provider's error body is useful in the log and never
            // reaches the client.
            let body = response.text().await.unwrap_or_default();
            Err(EmailError(format!(
                "the provider answered {status}: {body}"
            )))
        }
    }
}

/// How e-mail is sent and where the links in it point.
#[derive(Clone)]
pub struct EmailSettings {
    pub sender: Arc<dyn EmailSender>,
    /// Public address of the front-end, without a trailing slash.
    pub app_url: String,
}

impl EmailSettings {
    /// Reads the configuration documented in `.env.example`.
    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let get = |name: &str| get(name).filter(|value| !value.trim().is_empty());
        let development = match get("APP_ENV").as_deref() {
            None | Some("production") => false,
            Some("development") => true,
            Some(other) => {
                return Err(format!(
                    "APP_ENV must be development or production, got {other}"
                ));
            }
        };

        let app_url = match get("APP_URL") {
            Some(url) => url.trim_end_matches('/').to_string(),
            None if development => "http://localhost:3000".to_string(),
            None => return Err("APP_URL must be set (the public address of the front-end)".into()),
        };

        let sender: Arc<dyn EmailSender> = match get("EMAIL_PROVIDER").as_deref() {
            Some("resend") => {
                let api_key = get("EMAIL_API_KEY")
                    .ok_or("EMAIL_API_KEY must be set when EMAIL_PROVIDER is set")?;
                let from =
                    get("EMAIL_FROM").ok_or("EMAIL_FROM must be set when EMAIL_PROVIDER is set")?;
                let api_url = get("EMAIL_API_URL").unwrap_or_else(|| DEFAULT_API_URL.to_string());
                Arc::new(HttpEmailSender::new(&api_url, &api_key, &from)?)
            }
            Some(other) => return Err(format!("EMAIL_PROVIDER {other} is not supported")),
            None if development => Arc::new(LogEmailSender),
            None => {
                return Err(
                    "EMAIL_PROVIDER must be set: without a provider, e-mail is only logged, \
                     which is allowed in development (APP_ENV=development) alone"
                        .into(),
                );
            }
        };
        Ok(Self { sender, app_url })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn settings(pairs: &[(&str, &str)]) -> Result<EmailSettings, String> {
        let vars: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        EmailSettings::from_lookup(|name| vars.get(name).cloned())
    }

    #[test]
    fn development_without_a_provider_falls_back_to_logging() {
        let settings = settings(&[("APP_ENV", "development")]).unwrap();

        assert_eq!(settings.app_url, "http://localhost:3000");
    }

    #[test]
    fn production_without_a_provider_refuses_to_start() {
        let error = settings(&[("APP_URL", "https://tabwallet.app")])
            .err()
            .unwrap();

        assert!(error.contains("EMAIL_PROVIDER"), "{error}");
    }

    #[test]
    fn production_needs_the_public_address_of_the_front_end() {
        let error = settings(&[]).err().unwrap();

        assert!(error.contains("APP_URL"), "{error}");
    }

    #[test]
    fn a_provider_needs_its_key_and_sender_address() {
        let base = [
            ("APP_URL", "https://tabwallet.app/"),
            ("EMAIL_PROVIDER", "resend"),
        ];
        assert!(settings(&base).err().unwrap().contains("EMAIL_API_KEY"));

        let with_key = [base[0], base[1], ("EMAIL_API_KEY", "key")];
        assert!(settings(&with_key).err().unwrap().contains("EMAIL_FROM"));

        let complete = [
            with_key[0],
            with_key[1],
            with_key[2],
            ("EMAIL_FROM", "TabWallet <no-reply@tabwallet.app>"),
        ];
        assert_eq!(
            settings(&complete).unwrap().app_url,
            "https://tabwallet.app"
        );
    }

    #[test]
    fn an_unknown_provider_or_environment_is_rejected() {
        assert!(settings(&[("APP_ENV", "staging")]).is_err());
        assert!(
            settings(&[
                ("APP_ENV", "development"),
                ("EMAIL_PROVIDER", "carrier-pigeon")
            ])
            .is_err()
        );
    }
}
