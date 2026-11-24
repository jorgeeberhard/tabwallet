use crate::{auth::password_policy::Rejection, repositories::users::User};

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
    #[error("the e-mail and password match a sign-up that is not confirmed yet")]
    EmailNotConfirmed,
    #[error("the sign-up confirmation link is invalid, expired, used or replaced")]
    InvalidConfirmationToken,
    #[error("the current password is wrong")]
    InvalidCurrentPassword,
    #[error("the current password is required for this change")]
    CurrentPasswordRequired,
    #[error("the new password is refused by the Password policy: {}", .0.code())]
    PasswordRejected(Rejection),
    #[error("the password reset link is invalid, expired or already used")]
    InvalidResetToken,
    #[error("there is no pending e-mail change to resend")]
    NoPendingEmailChange,
    #[error("too many requests; try again in {retry_after_secs} seconds")]
    TooManyRequests { retry_after_secs: u64 },
    #[error("there is no valid session")]
    Unauthenticated,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

internal_error!(AuthError);
