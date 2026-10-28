use axum::{Json, extract::State};
use serde::Deserialize;
use validator::Validate;

use super::{
    auth::{MAX_PASSWORD_LEN, MIN_PASSWORD_LEN, UserEnvelope},
    extract::ValidatedJson,
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    repositories::users::UserRepository,
    use_cases::{ProfileChanges, UpdateProfileUseCase},
};

pub const MAX_NAME_LEN: u64 = 100;

/// Every field is optional: only what is present changes. Changing `email` or
/// `new_password` also needs `current_password`.
#[derive(Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[serde(default, deserialize_with = "trimmed")]
    #[validate(length(max = MAX_NAME_LEN, code = "name_length"))]
    name: Option<String>,
    #[serde(default, deserialize_with = "trimmed")]
    #[validate(email(code = "invalid_email"))]
    email: Option<String>,
    #[validate(length(min = MIN_PASSWORD_LEN, max = MAX_PASSWORD_LEN, code = "password_length"))]
    new_password: Option<String>,
    current_password: Option<String>,
}

/// Surrounding spaces are a copy-and-paste accident, not part of the value.
fn trimmed<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer).map(|value| value.map(|v| v.trim().to_string()))
}

/// `PATCH /profile` (protected): edits name, e-mail and password.
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpdateProfileRequest>,
) -> Result<Json<UserEnvelope>, ApiError> {
    let changes = ProfileChanges {
        name: request.name,
        email: request.email,
        new_password: request.new_password,
        current_password: request.current_password,
    };
    let user = UpdateProfileUseCase::new(UserRepository::new(state.pool.clone()))
        .execute(auth_user.id, changes)
        .await?;
    Ok(Json(UserEnvelope { user: user.into() }))
}
