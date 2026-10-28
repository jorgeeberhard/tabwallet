use axum::{extract::State, http::StatusCode};
use serde::Deserialize;
use validator::Validate;

use super::{
    auth::{MAX_PASSWORD_LEN, MIN_PASSWORD_LEN},
    extract::ValidatedJson,
};
use crate::{
    app::AppState,
    error::ApiError,
    repositories::{password_resets::PasswordResetRepository, users::UserRepository},
    use_cases::{RequestPasswordResetUseCase, ResetPasswordUseCase},
};

#[derive(Deserialize, Validate)]
pub struct RequestResetRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(email(code = "invalid_email"))]
    email: String,
}

#[derive(Deserialize, Validate)]
pub struct ConfirmResetRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, code = "required"))]
    token: String,
    #[validate(length(min = MIN_PASSWORD_LEN, max = MAX_PASSWORD_LEN, code = "password_length"))]
    password: String,
}

fn trimmed<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    String::deserialize(deserializer).map(|value| value.trim().to_string())
}

/// `POST /auth/password-reset/request`: e-mails a reset link. The answer is
/// `202` whether or not the address has an account.
pub async fn request(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<RequestResetRequest>,
) -> Result<StatusCode, ApiError> {
    RequestPasswordResetUseCase::new(
        UserRepository::new(state.pool.clone()),
        PasswordResetRepository::new(state.pool.clone()),
        state.email.clone(),
        state.clock.clone(),
        state.app_url.clone(),
    )
    .execute(&request.email)
    .await?;
    Ok(StatusCode::ACCEPTED)
}

/// `POST /auth/password-reset/confirm`: sets the new password with the token
/// from the e-mailed link. Does not sign anyone in.
pub async fn confirm(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<ConfirmResetRequest>,
) -> Result<StatusCode, ApiError> {
    ResetPasswordUseCase::new(
        PasswordResetRepository::new(state.pool.clone()),
        state.clock.clone(),
    )
    .execute(&request.token, &request.password)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
