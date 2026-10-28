use crate::repositories::users::User;

/// A signed-in user and the token that proves it.
pub struct Session {
    pub user: User,
    pub token: String,
}

/// Why an account use case failed. Unaware of HTTP: the handler layer decides
/// the status code.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("an account with this e-mail already exists")]
    EmailTaken,
    #[error("wrong e-mail or password")]
    InvalidCredentials,
    #[error("there is no valid session")]
    Unauthenticated,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl AuthError {
    pub fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}
