use std::sync::Arc;

use chrono::Duration;

use crate::{
    auth::{
        self, link_token,
        password_policy::{self, Account},
    },
    clock::Clock,
    email::EmailSender,
    repositories::{pending_signups::PendingSignupRepository, users::UserRepository},
};

use super::{
    AuthError,
    sign_up_messages::{self, CONFIRMATION_LINK_TTL_HOURS},
};

/// UC01, first half: asks for an account. Nothing but a Pending signup is
/// created, and nobody is signed in: the account exists once the link in the
/// e-mail is opened (see `ConfirmSignUpUseCase`).
pub struct SignUpUseCase {
    users: UserRepository,
    pending: PendingSignupRepository,
    sender: Arc<dyn EmailSender>,
    clock: Arc<dyn Clock>,
    app_url: String,
}

impl SignUpUseCase {
    pub fn new(
        users: UserRepository,
        pending: PendingSignupRepository,
        sender: Arc<dyn EmailSender>,
        clock: Arc<dyn Clock>,
        app_url: String,
    ) -> Self {
        Self {
            users,
            pending,
            sender,
            clock,
            app_url,
        }
    }

    /// `email` and `name` are already validated by the handler. The Password
    /// policy judges the password against them, which tells nothing about
    /// whether the e-mail has an account.
    ///
    /// Succeeds, and looks the same to the caller, whether the e-mail is new,
    /// has a Pending signup (replaced: new name, password and link) or already
    /// has an account (which only gets a notice and is left untouched). Both
    /// outcomes do the same work (hash the password, clean expired Pending
    /// signups, write or read, send one e-mail), so neither the answer nor
    /// the time it takes reveals which e-mails have an account. A failure to
    /// deliver the e-mail is only logged, for the same reason.
    pub async fn execute(&self, email: &str, name: &str, password: &str) -> Result<(), AuthError> {
        password_policy::check(password, Account { email, name })
            .map_err(AuthError::PasswordRejected)?;
        let password_hash = auth::password::hash(password.to_string())
            .await
            .map_err(AuthError::Internal)?;
        let now = self.clock.now();
        self.pending
            .delete_expired(now)
            .await
            .map_err(AuthError::internal)?;

        let existing = self
            .users
            .find_credentials_by_email(email)
            .await
            .map_err(AuthError::internal)?;
        let message = match existing {
            Some(account) => sign_up_messages::account_already_exists(
                &account.user.email,
                &account.user.name,
                &self.app_url,
            ),
            None => {
                let token = link_token::generate();
                self.pending
                    .replace(
                        email,
                        name,
                        &password_hash,
                        &link_token::hash(&token),
                        now,
                        now + Duration::hours(CONFIRMATION_LINK_TTL_HOURS),
                    )
                    .await
                    .map_err(AuthError::internal)?;
                sign_up_messages::confirmation(email, name, &self.app_url, &token)
            }
        };
        if let Err(error) = self.sender.send(message).await {
            tracing::error!("sign-up e-mail was not delivered: {error}");
        }
        Ok(())
    }
}
