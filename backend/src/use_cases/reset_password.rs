use std::sync::Arc;

use crate::{
    auth::{
        self, link_token,
        password_policy::{self, Account},
    },
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

    /// An unknown, expired or used token fails first, the same way for all of
    /// them. Then the Password policy judges `new_password` against the
    /// e-mail and name of the account the link belongs to; a refusal leaves
    /// the link usable. Sessions already open stay valid: the session token is
    /// a JWT, which cannot be revoked.
    pub async fn execute(&self, token: &str, new_password: &str) -> Result<(), AuthError> {
        let token_hash = link_token::hash(token);
        let (email, name) = self
            .resets
            .find_account(&token_hash, self.clock.now())
            .await
            .map_err(AuthError::internal)?
            .ok_or(AuthError::InvalidResetToken)?;
        password_policy::check(
            new_password,
            Account {
                email: &email,
                name: &name,
            },
        )
        .map_err(AuthError::PasswordRejected)?;
        let password_hash = auth::password::hash(new_password.to_string())
            .await
            .map_err(AuthError::Internal)?;
        let done = self
            .resets
            .reset_password(&token_hash, self.clock.now(), &password_hash)
            .await
            .map_err(AuthError::internal)?;
        if done {
            Ok(())
        } else {
            Err(AuthError::InvalidResetToken)
        }
    }
}
