use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

use super::{
    extract::{ValidatedJson, trimmed},
    profile::{MAX_NAME_LEN, ProfileEnvelope},
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::UserId,
    repositories::{
        pending_email_changes::PendingEmailChangeRepository,
        pending_signups::PendingSignupRepository,
        users::{User, UserRepository},
    },
    use_cases::{
        AuthError, ConfirmSignUpUseCase, GetCurrentUserUseCase, LogInUseCase,
        ResendConfirmationUseCase, SignUpUseCase,
    },
};

#[derive(Deserialize, Validate)]
pub struct SignUpRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(email(code = "invalid_email"))]
    email: String,
    #[serde(deserialize_with = "trimmed")]
    #[validate(custom(function = "validate_name"))]
    name: String,
    /// Judged by the Password policy in the use case, which also knows the
    /// e-mail and the name.
    password: String,
}

/// The name is required at sign-up (it is already trimmed) and at most
/// `MAX_NAME_LEN` characters, like in the profile.
fn validate_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::new("required"));
    }
    if name.chars().count() as u64 > MAX_NAME_LEN {
        return Err(ValidationError::new("name_length"));
    }
    Ok(())
}

#[derive(Deserialize, Validate)]
pub struct ConfirmSignUpRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, code = "required"))]
    token: String,
}

#[derive(Deserialize, Validate)]
pub struct ResendConfirmationRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(email(code = "invalid_email"))]
    email: String,
}

#[derive(Deserialize, Validate)]
pub struct LogInRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, code = "required"))]
    email: String,
    #[validate(length(min = 1, code = "required"))]
    password: String,
}

/// The public view of a user.
#[derive(Serialize)]
pub struct UserResponse {
    id: UserId,
    email: String,
    name: String,
    created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            created_at: user.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct UserEnvelope {
    pub user: UserResponse,
}

/// `POST /auth/login`: checks the credentials and starts a session.
pub async fn log_in(
    State(state): State<AppState>,
    jar: CookieJar,
    ValidatedJson(request): ValidatedJson<LogInRequest>,
) -> Result<(CookieJar, Json<UserEnvelope>), ApiError> {
    let use_case = LogInUseCase::new(
        UserRepository::new(state.pool.clone()),
        PendingSignupRepository::new(state.pool.clone()),
        state.auth.tokens().clone(),
        state.clock.clone(),
    );
    // Wrong passwords are counted per e-mail address; the limit applies to
    // unknown addresses too, so it does not reveal which have an account.
    let key = request.email.to_lowercase();
    let now = state.clock.now();
    state
        .rate_limits
        .login
        .check(&key, now)
        .map_err(|retry_after_secs| ApiError::TooManyRequests { retry_after_secs })?;
    let session = match use_case.execute(&request.email, &request.password).await {
        Ok(session) => session,
        Err(AuthError::InvalidCredentials) => {
            state.rate_limits.login.record(&key, now);
            return Err(AuthError::InvalidCredentials.into());
        }
        Err(error) => return Err(error.into()),
    };
    state.rate_limits.login.clear(&key);
    Ok((
        jar.add(state.auth.session_cookie(session.token)),
        Json(UserEnvelope {
            user: session.user.into(),
        }),
    ))
}

/// `POST /auth/logout`: ends the session in the browser. Works without a
/// session too, so a stale cookie can always be cleared.
pub async fn log_out(State(state): State<AppState>, jar: CookieJar) -> (StatusCode, CookieJar) {
    (StatusCode::NO_CONTENT, jar.add(state.auth.removal_cookie()))
}

/// `GET /auth/me` (protected): the signed-in user and the e-mail change
/// waiting for confirmation, if any.
pub async fn me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<ProfileEnvelope>, ApiError> {
    let view = GetCurrentUserUseCase::new(
        UserRepository::new(state.pool.clone()),
        PendingEmailChangeRepository::new(state.pool.clone()),
        state.clock.clone(),
    )
    .execute(auth_user.id)
    .await?;
    Ok(Json(view.into()))
}

/// `POST /auth/signup`: asks for an account. It creates (or replaces) a
/// Pending signup and e-mails the Confirmation link; it never starts a
/// session. The answer is `202` whether the e-mail is new, pending or already
/// has an account.
pub async fn sign_up(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<SignUpRequest>,
) -> Result<StatusCode, ApiError> {
    // Every accepted request counts, for any address, so the limit reveals
    // nothing and nobody can flood a mailbox with sign-up e-mails. A password
    // the policy refuses sends nothing, so it does not count.
    let key = request.email.to_lowercase();
    let now = state.clock.now();
    state
        .rate_limits
        .sign_up
        .check(&key, now)
        .map_err(|retry_after_secs| ApiError::TooManyRequests { retry_after_secs })?;
    let result = SignUpUseCase::new(
        UserRepository::new(state.pool.clone()),
        PendingSignupRepository::new(state.pool.clone()),
        state.email.clone(),
        state.clock.clone(),
        state.app_url.clone(),
    )
    .execute(&request.email, &request.name, &request.password)
    .await;
    if !matches!(result, Err(AuthError::PasswordRejected(_))) {
        state.rate_limits.sign_up.record(&key, now);
    }
    result?;
    Ok(StatusCode::ACCEPTED)
}

/// `POST /auth/signup/confirm`: opens the Confirmation link: creates the
/// account. Does not sign anyone in.
pub async fn confirm_sign_up(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<ConfirmSignUpRequest>,
) -> Result<StatusCode, ApiError> {
    ConfirmSignUpUseCase::new(
        PendingSignupRepository::new(state.pool.clone()),
        state.clock.clone(),
    )
    .execute(&request.token)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /auth/signup/resend`: e-mails a new Confirmation link. The answer is
/// `202` whether or not the address has a Pending signup.
pub async fn resend_confirmation(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<ResendConfirmationRequest>,
) -> Result<StatusCode, ApiError> {
    let key = request.email.to_lowercase();
    let now = state.clock.now();
    state
        .rate_limits
        .resend_confirmation
        .check(&key, now)
        .map_err(|retry_after_secs| ApiError::TooManyRequests { retry_after_secs })?;
    state.rate_limits.resend_confirmation.record(&key, now);
    ResendConfirmationUseCase::new(
        PendingSignupRepository::new(state.pool.clone()),
        state.email.clone(),
        state.clock.clone(),
        state.app_url.clone(),
    )
    .execute(&request.email)
    .await?;
    Ok(StatusCode::ACCEPTED)
}
