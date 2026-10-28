use std::sync::Arc;

use crate::{
    auth::{self, reset_token},
    clock::Clock,
    repositories::password_resets::PasswordResetRepository,
};

use super::AuthError;

/// UC03, second half: sets a new password with the link from the e-mail.
pub struct ResetPasswordUseCase {
    resets: PasswordResetRepository,
    clock: Arc<dyn Clock>,
}

impl ResetPasswordUseCase {
    pub fn new(resets: PasswordResetRepository, clock: Arc<dyn Clock>) -> Self {
        Self { resets, clock }
    }

    /// `new_password` is already validated by the handler. A token that is
    /// unknown, expired or already used fails the same way. Sessions already
    /// open stay valid: the session token is a JWT, which cannot be revoked.
    pub async fn execute(&self, token: &str, new_password: &str) -> Result<(), AuthError> {
        let password_hash = auth::password::hash(new_password.to_string())
            .await
            .map_err(AuthError::Internal)?;
        let done = self
            .resets
            .reset_password(&reset_token::hash(token), self.clock.now(), &password_hash)
            .await
            .map_err(AuthError::internal)?;
        if done {
            Ok(())
        } else {
            Err(AuthError::InvalidResetToken)
        }
    }
}
