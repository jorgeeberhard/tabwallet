use std::sync::Arc;

use chrono::Duration;

use crate::{
    auth::link_token,
    clock::Clock,
    email::EmailSender,
    ids::UserId,
    rate_limit::RateLimiter,
    repositories::{
        pending_email_changes::{ConfirmError, PendingEmailChangeRepository},
        users::UserRepository,
    },
};

use super::{
    AuthError, email_change_messages,
    sign_up_messages::{self, CONFIRMATION_LINK_TTL_HOURS},
};

/// UC14, second half: opens the Confirmation link sent to the new e-mail and
/// gives the account that e-mail.
pub struct ConfirmEmailChangeUseCase {
    pending: PendingEmailChangeRepository,
    sender: Arc<dyn EmailSender>,
    clock: Arc<dyn Clock>,
    app_url: String,
}

impl ConfirmEmailChangeUseCase {
    pub fn new(
        pending: PendingEmailChangeRepository,
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

    /// Applies the change and deletes the Pending e-mail change in one
    /// transaction, then tells the OLD address ("your access e-mail was
    /// changed"). An unknown, expired, used or replaced link fails the same
    /// way. Refuses when the new e-mail became an account meanwhile, and then
    /// the account keeps its e-mail and nothing is sent. It needs no session
    /// and never starts one; sessions already open stay valid. A failure to
    /// deliver the notice is only logged: the change is done.
    pub async fn execute(&self, token: &str) -> Result<(), AuthError> {
        let change = self
            .pending
            .confirm(&link_token::hash(token), self.clock.now())
            .await
            .map_err(|error| match error {
                ConfirmError::InvalidToken => AuthError::InvalidConfirmationToken,
                ConfirmError::EmailTaken => AuthError::EmailTaken,
                ConfirmError::Database(error) => AuthError::internal(error),
            })?;
        let notice = email_change_messages::old_address_notice(
            &change.old_email,
            &change.name,
            &change.new_email,
            &self.app_url,
        );
        if let Err(error) = self.sender.send(notice).await {
            tracing::error!("e-mail change notice was not delivered: {error}");
        }
        Ok(())
    }
}

/// Sends the link of the account's Pending e-mail change again.
pub struct ResendEmailChangeUseCase {
    users: UserRepository,
    pending: PendingEmailChangeRepository,
    sender: Arc<dyn EmailSender>,
    clock: Arc<dyn Clock>,
    app_url: String,
    /// The same limiter as the requests to change the e-mail.
    limiter: RateLimiter,
}

impl ResendEmailChangeUseCase {
    pub fn new(
        users: UserRepository,
        pending: PendingEmailChangeRepository,
        sender: Arc<dyn EmailSender>,
        clock: Arc<dyn Clock>,
        app_url: String,
        limiter: RateLimiter,
    ) -> Self {
        Self {
            users,
            pending,
            sender,
            clock,
            app_url,
            limiter,
        }
    }

    /// Gives the live Pending e-mail change a new link (the previous one stops
    /// working) and e-mails it to the pending address; counts in the
    /// account's limit of 3 an hour. `NoPendingEmailChange` when there is none
    /// (never asked, confirmed, or expired). When the address has an account
    /// by now, it gets the "you already have an account" notice instead, like
    /// at the request, so the answer reveals nothing. A failure to deliver the
    /// e-mail is only logged.
    pub async fn execute(&self, user_id: UserId) -> Result<(), AuthError> {
        let now = self.clock.now();
        let limit_key = user_id.0.to_string();
        self.limiter
            .check(&limit_key, now)
            .map_err(|retry_after_secs| AuthError::TooManyRequests { retry_after_secs })?;
        let user = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(AuthError::internal)?
            .ok_or(AuthError::Unauthenticated)?;
        let token = link_token::generate();
        let new_email = self
            .pending
            .renew_link(
                user_id,
                &link_token::hash(&token),
                now,
                now + Duration::hours(CONFIRMATION_LINK_TTL_HOURS),
            )
            .await
            .map_err(AuthError::internal)?
            .ok_or(AuthError::NoPendingEmailChange)?;
        self.limiter.record(&limit_key, now);

        let owner = self
            .users
            .find_credentials_by_email(&new_email)
            .await
            .map_err(AuthError::internal)?;
        let message = match owner {
            Some(account) => sign_up_messages::account_already_exists(
                &account.user.email,
                &account.user.name,
                &self.app_url,
            ),
            None => {
                email_change_messages::confirmation(&new_email, &user.name, &self.app_url, &token)
            }
        };
        if let Err(error) = self.sender.send(message).await {
            tracing::error!("e-mail change message was not delivered: {error}");
        }
        Ok(())
    }
}
