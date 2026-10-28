//! The HTTP error type and its stable JSON shape.
//!
//! Every error response looks like
//! `{"error": {"code": "email_taken", "message": "...", "fields": {...}}}`.
//! The front-end switches on `code` (stable) and shows its own Portuguese
//! text; `message` is English, for logs and developers; `fields` appears only
//! for validation errors and maps a field name to a rule code.

use std::collections::BTreeMap;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use validator::ValidationErrors;

use crate::use_cases::AuthError;

#[derive(Debug)]
pub enum ApiError {
    /// The body is not the JSON the route expects.
    InvalidRequest,
    /// The body parsed but broke a validation rule.
    Validation(BTreeMap<String, String>),
    EmailTaken,
    InvalidCredentials,
    /// No valid session.
    Unauthenticated,
    /// Something unexpected; the cause is logged, never sent to the client.
    Internal(String),
}

impl ApiError {
    pub fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(errors: ValidationErrors) -> Self {
        let fields = errors
            .field_errors()
            .into_iter()
            .filter_map(|(field, rules)| {
                rules
                    .first()
                    .map(|rule| (field.to_string(), rule.code.to_string()))
            })
            .collect();
        Self::Validation(fields)
    }
}

impl From<AuthError> for ApiError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::EmailTaken => Self::EmailTaken,
            AuthError::InvalidCredentials => Self::InvalidCredentials,
            AuthError::Unauthenticated => Self::Unauthenticated,
            AuthError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message, fields) = match self {
            Self::InvalidRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request body is not valid JSON for this route.",
                None,
            ),
            Self::Validation(fields) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "One or more fields are invalid.",
                Some(fields),
            ),
            Self::EmailTaken => (
                StatusCode::CONFLICT,
                "email_taken",
                "An account with this e-mail already exists.",
                None,
            ),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Wrong e-mail or password.",
                None,
            ),
            Self::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Sign in to continue.",
                None,
            ),
            Self::Internal(cause) => {
                tracing::error!("internal error: {cause}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Something went wrong.",
                    None,
                )
            }
        };
        let mut error = json!({ "code": code, "message": message });
        if let Some(fields) = fields {
            error["fields"] = json!(fields);
        }
        (status, Json(json!({ "error": error }))).into_response()
    }
}
