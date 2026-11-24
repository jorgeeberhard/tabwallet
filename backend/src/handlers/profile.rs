use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use std::collections::BTreeMap;

use super::{auth::UserResponse, extract::ValidatedJson};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    repositories::{
        pending_email_changes::{PendingEmailChange, PendingEmailChangeRepository},
        users::UserRepository,
    },
    use_cases::{
        AuthError, ConfirmEmailChangeUseCase, ProfileChanges, ProfileView,
        ResendEmailChangeUseCase, UpdateProfileUseCase,
    },
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
    /// Judged by the Password policy in the use case, which also knows the
    /// account's e-mail and name.
    new_password: Option<String>,
    current_password: Option<String>,
}

#[derive(Deserialize, Validate)]
pub struct ConfirmEmailChangeRequest {
    #[serde(deserialize_with = "super::extract::trimmed")]
    #[validate(length(min = 1, code = "required"))]
    token: String,
}

/// The e-mail change that waits for the new address to be confirmed.
#[derive(Serialize)]
pub struct PendingEmailChangeResponse {
    /// The new address, as typed.
    email: String,
    /// When the link sent to it stops working.
    expires_at: DateTime<Utc>,
}

impl From<PendingEmailChange> for PendingEmailChangeResponse {
    fn from(change: PendingEmailChange) -> Self {
        Self {
            email: change.new_email,
            expires_at: change.expires_at,
        }
    }
}

/// The signed-in user and the e-mail change waiting for confirmation, if any.
/// `GET /auth/me` and `PATCH /profile` answer with it.
#[derive(Serialize)]
pub struct ProfileEnvelope {
    pub user: UserResponse,
    pub pending_email_change: Option<PendingEmailChangeResponse>,
}

impl From<ProfileView> for ProfileEnvelope {
    fn from(view: ProfileView) -> Self {
        Self {
            user: view.user.into(),
            pending_email_change: view.pending_email_change.map(Into::into),
        }
    }
}

/// Surrounding spaces are a copy-and-paste accident, not part of the value.
fn trimmed<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer).map(|value| value.map(|v| v.trim().to_string()))
}

/// `PATCH /profile` (protected): edits name, e-mail and password. A new e-mail
/// is not applied: it becomes a pending change, confirmed by the link sent to
/// the new address (the answer says which address is pending).
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpdateProfileRequest>,
) -> Result<Json<ProfileEnvelope>, ApiError> {
    let changes = ProfileChanges {
        name: request.name,
        email: request.email,
        new_password: request.new_password,
        current_password: request.current_password,
    };
    let view = UpdateProfileUseCase::new(
        UserRepository::new(state.pool.clone()),
        PendingEmailChangeRepository::new(state.pool.clone()),
        state.email.clone(),
        state.clock.clone(),
        state.app_url.clone(),
        state.rate_limits.email_change.clone(),
    )
    .execute(auth_user.id, changes)
    .await
    .map_err(|error| match error {
        AuthError::PasswordRejected(rejection) => ApiError::Validation(BTreeMap::from([(
            "new_password".to_string(),
            rejection.code().to_string(),
        )])),
        other => other.into(),
    })?;
    Ok(Json(view.into()))
}

/// `POST /profile/email-change/resend` (protected): e-mails the link of the
/// pending e-mail change again; the previous link stops working. `202`, or
/// `404 no_pending_email_change` when there is none. Shares the limit of 3 an
/// hour with the requests to change the e-mail.
pub async fn resend_email_change(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<StatusCode, ApiError> {
    ResendEmailChangeUseCase::new(
        UserRepository::new(state.pool.clone()),
        PendingEmailChangeRepository::new(state.pool.clone()),
        state.email.clone(),
        state.clock.clone(),
        state.app_url.clone(),
        state.rate_limits.email_change.clone(),
    )
    .execute(auth_user.id)
    .await?;
    Ok(StatusCode::ACCEPTED)
}

/// `POST /auth/email-change/confirm`: opens the link sent to the new address
/// and gives the account that e-mail. Public like the other links: it needs no
/// session, and signs nobody in.
pub async fn confirm_email_change(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<ConfirmEmailChangeRequest>,
) -> Result<StatusCode, ApiError> {
    ConfirmEmailChangeUseCase::new(
        PendingEmailChangeRepository::new(state.pool.clone()),
        state.email.clone(),
        state.clock.clone(),
        state.app_url.clone(),
    )
    .execute(&request.token)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
