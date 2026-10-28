use axum::{Json, extract::State};
use serde::Deserialize;
use validator::Validate;

use super::{auth::UserEnvelope, extract::ValidatedJson};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    repositories::users::UserRepository,
    use_cases::{ProfileChanges, UpdateProfileUseCase},
};

pub const MAX_NAME_LEN: u64 = 100;

/// Every field is optional: only what is present changes.
#[derive(Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[serde(default, deserialize_with = "trimmed")]
    #[validate(length(max = MAX_NAME_LEN, code = "name_length"))]
    name: Option<String>,
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
    let user = UpdateProfileUseCase::new(UserRepository::new(state.pool.clone()))
        .execute(auth_user.id, ProfileChanges { name: request.name })
        .await?;
    Ok(Json(UserEnvelope { user: user.into() }))
}
