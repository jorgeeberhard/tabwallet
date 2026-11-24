use std::sync::Arc;

use chrono::Duration;

use crate::{
    auth::link_token,
    clock::Clock,
    email::EmailSender,
    repositories::pending_signups::{ConfirmError, PendingSignupRepository},
};

use super::{
    AuthError,
    sign_up_messages::{self, CONFIRMATION_LINK_TTL_HOURS},
};

/// UC01, second half: opens the Confirmation link and creates the account.
pub struct ConfirmSignUpUseCase {
    pending: PendingSignupRepository,
    clock: Arc<dyn Clock>,
}

impl ConfirmSignUpUseCase {
    pub fn new(pending: PendingSignupRepository, clock: Arc<dyn Clock>) -> Self {
        Self { pending, clock }
    }

    /// Creates the account and its default categories and deletes the
    /// Pending signup, in one transaction. An unknown, expired, used or
    /// replaced link fails the same way. Refuses when the e-mail became an
    /// account in the meantime. It never starts a session: the person logs
    /// in with the password chosen at sign-up.
    pub async fn execute(&self, token: &str) -> Result<(), AuthError> {
        self.pending
            .confirm(&link_token::hash(token), self.clock.now())
            .await
            .map(|_user| ())
            .map_err(|error| match error {
                ConfirmError::InvalidToken => AuthError::InvalidConfirmationToken,
                ConfirmError::EmailTaken => AuthError::EmailTaken,
                ConfirmError::Database(error) => AuthError::internal(error),
            })
    }
}

/// Sends the confirmation e-mail again, with a new link.
pub struct ResendConfirmationUseCase {
    pending: PendingSignupRepository,
    sender: Arc<dyn EmailSender>,
    clock: Arc<dyn Clock>,
    app_url: String,
}

impl ResendConfirmationUseCase {
    pub fn new(
        pending: PendingSignupRepository,
        sender: Arc<dyn EmailSender>,
        clock: Arc<dyn Clock>,
        app_url: String,
    ) -> Self {
        Self {
            pending,
            sender,
            clock,
            app_url,
        }
    }

    /// Gives the live Pending signup of the e-mail a new link (the previous
    /// one stops working) and e-mails it; the password and name stay. Succeeds
    /// and looks the same when there is no live Pending signup (none, expired,
    /// or the e-mail has an account), and also when the provider fails, so the
    /// answer reveals nothing about the e-mail.
    pub async fn execute(&self, email: &str) -> Result<(), AuthError> {
        let now = self.clock.now();
        let token = link_token::generate();
        let renewed = self
            .pending
            .renew_link(
                email,
                &link_token::hash(&token),
                now,
                now + Duration::hours(CONFIRMATION_LINK_TTL_HOURS),
            )
            .await
            .map_err(AuthError::internal)?;
        let Some((stored_email, name)) = renewed else {
            return Ok(());
        };
        let message = sign_up_messages::confirmation(&stored_email, &name, &self.app_url, &token);
        if let Err(error) = self.sender.send(message).await {
            tracing::error!("confirmation e-mail was not delivered: {error}");
        }
        Ok(())
    }
}
