use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::extract::{ValidatedJson, trimmed};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::UserId,
    repositories::users::{User, UserRepository},
    use_cases::{GetCurrentUserUseCase, LogInUseCase, SignUpUseCase},
};

pub const MIN_PASSWORD_LEN: u64 = 8;
/// Upper bound that keeps a huge body from becoming a huge Argon2 job.
pub const MAX_PASSWORD_LEN: u64 = 128;

#[derive(Deserialize, Validate)]
pub struct SignUpRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(email(code = "invalid_email"))]
    email: String,
    #[validate(length(min = MIN_PASSWORD_LEN, max = MAX_PASSWORD_LEN, code = "password_length"))]
    password: String,
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
    created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct UserEnvelope {
    user: UserResponse,
}

/// `POST /auth/login`: checks the credentials and starts a session.
pub async fn log_in(
    State(state): State<AppState>,
    jar: CookieJar,
    ValidatedJson(request): ValidatedJson<LogInRequest>,
) -> Result<(CookieJar, Json<UserEnvelope>), ApiError> {
    let use_case = LogInUseCase::new(
        UserRepository::new(state.pool.clone()),
        state.auth.tokens().clone(),
    );
    let session = use_case.execute(&request.email, &request.password).await?;
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

/// `GET /auth/me` (protected): the signed-in user.
pub async fn me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<UserEnvelope>, ApiError> {
    let user = GetCurrentUserUseCase::new(UserRepository::new(state.pool.clone()))
        .execute(auth_user.id)
        .await?;
    Ok(Json(UserEnvelope { user: user.into() }))
}

/// `POST /auth/signup`: creates the account and starts a session.
pub async fn sign_up(
    State(state): State<AppState>,
    jar: CookieJar,
    ValidatedJson(request): ValidatedJson<SignUpRequest>,
) -> Result<(StatusCode, CookieJar, Json<UserEnvelope>), ApiError> {
    let use_case = SignUpUseCase::new(
        UserRepository::new(state.pool.clone()),
        state.auth.tokens().clone(),
    );
    let session = use_case.execute(&request.email, &request.password).await?;
    Ok((
        StatusCode::CREATED,
        jar.add(state.auth.session_cookie(session.token)),
        Json(UserEnvelope {
            user: session.user.into(),
        }),
    ))
}
