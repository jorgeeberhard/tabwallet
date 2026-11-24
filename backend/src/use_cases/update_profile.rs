use std::sync::Arc;

use chrono::Duration;

use crate::{
    auth::{
        self, link_token,
        password_policy::{self, Account},
    },
    clock::Clock,
    email::EmailSender,
    ids::UserId,
    rate_limit::RateLimiter,
    repositories::{
        pending_email_changes::{PendingEmailChange, PendingEmailChangeRepository},
        users::{InsertUserError, User, UserRepository},
    },
};

use super::{
    AuthError, email_change_messages,
    sign_up_messages::{self, CONFIRMATION_LINK_TTL_HOURS},
};

/// What the user asked to change. `None` leaves a field as it is.
pub struct ProfileChanges {
    pub name: Option<String>,
    /// A different e-mail is not applied: it becomes a Pending e-mail change
    /// that waits for the new address to be confirmed.
    pub email: Option<String>,
    pub new_password: Option<String>,
    /// Proves who is asking; required to change the e-mail or the password.
    pub current_password: Option<String>,
}

/// The profile after an update: the user as stored (the e-mail is still the
/// old one while a change waits) and the live Pending e-mail change, if any.
pub struct ProfileView {
    pub user: User,
    pub pending_email_change: Option<PendingEmailChange>,
}

/// UC14: edits the profile.
pub struct UpdateProfileUseCase {
    users: UserRepository,
    pending: PendingEmailChangeRepository,
    sender: Arc<dyn EmailSender>,
    clock: Arc<dyn Clock>,
    app_url: String,
    /// Requests to change the e-mail and their resends, per account.
    limiter: RateLimiter,
}

impl UpdateProfileUseCase {
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

    /// The fields are already validated by the handler. A new password is
    /// judged by the Password policy against the e-mail and name the account
    /// has now (not the ones in this same request), before the current
    /// password is checked.
    ///
    /// Name and password apply at once. An e-mail that differs from the
    /// current one (ignoring letter case) does not: it creates, or replaces,
    /// the account's Pending e-mail change and sends the Confirmation link to
    /// the new address, so the old e-mail keeps logging in until the link is
    /// opened. It needs the current password and counts in the account's
    /// limit of 3 requests (and resends) an hour; a refused request (limit,
    /// password, policy) changes nothing at all.
    ///
    /// When the new address already has an account, the answer is the same
    /// and a Pending e-mail change is stored the same, but the address gets
    /// the "you already have an account" notice instead of a link (nobody
    /// ever holds its token), so the person asking cannot tell which
    /// addresses have an account.
    ///
    /// Sessions already open stay valid after an e-mail or password change:
    /// the session token is a JWT, which cannot be revoked (see README).
    pub async fn execute(
        &self,
        user_id: UserId,
        changes: ProfileChanges,
    ) -> Result<ProfileView, AuthError> {
        let current = self
            .users
            .find_credentials_by_id(user_id)
            .await
            .map_err(AuthError::internal)?
            .ok_or(AuthError::Unauthenticated)?;

        if let Some(new_password) = &changes.new_password {
            password_policy::check(
                new_password,
                Account {
                    email: &current.user.email,
                    name: &current.user.name,
                },
            )
            .map_err(AuthError::PasswordRejected)?;
        }

        let new_email = changes
            .email
            .filter(|email| !email.eq_ignore_ascii_case(&current.user.email));
        let is_sensitive = new_email.is_some() || changes.new_password.is_some();
        if is_sensitive {
            let given = changes
                .current_password
                .ok_or(AuthError::CurrentPasswordRequired)?;
            if !auth::password::verify(given, current.password_hash.clone()).await {
                return Err(AuthError::InvalidCurrentPassword);
            }
        }
        let now = self.clock.now();
        let limit_key = user_id.0.to_string();
        if new_email.is_some() {
            self.limiter
                .check(&limit_key, now)
                .map_err(|retry_after_secs| AuthError::TooManyRequests { retry_after_secs })?;
        }

        let new_password_hash = match changes.new_password {
            Some(password) => Some(
                auth::password::hash(password)
                    .await
                    .map_err(AuthError::Internal)?,
            ),
            None => None,
        };
        let name = changes.name.unwrap_or(current.user.name);
        let user = self
            .users
            .update_profile(
                user_id,
                &name,
                &current.user.email,
                new_password_hash.as_deref(),
            )
            .await
            .map_err(|error| match error {
                InsertUserError::EmailTaken => AuthError::EmailTaken,
                InsertUserError::Database(error) => AuthError::internal(error),
            })?;

        if let Some(new_email) = new_email {
            self.request_email_change(user_id, &user, &new_email)
                .await?;
            self.limiter.record(&limit_key, now);
        }
        let pending_email_change = self
            .pending
            .find_live(user_id, now)
            .await
            .map_err(AuthError::internal)?;
        Ok(ProfileView {
            user,
            pending_email_change,
        })
    }

    /// Stores the Pending e-mail change and sends the one e-mail it calls
    /// for. A failure to deliver it is only logged, for the same reason as at
    /// sign-up.
    async fn request_email_change(
        &self,
        user_id: UserId,
        user: &User,
        new_email: &str,
    ) -> Result<(), AuthError> {
        let now = self.clock.now();
        self.pending
            .delete_expired(now)
            .await
            .map_err(AuthError::internal)?;
        let owner = self
            .users
            .find_credentials_by_email(new_email)
            .await
            .map_err(AuthError::internal)?;
        let token = link_token::generate();
        self.pending
            .replace(
                user_id,
                new_email,
                &link_token::hash(&token),
                now,
                now + Duration::hours(CONFIRMATION_LINK_TTL_HOURS),
            )
            .await
            .map_err(AuthError::internal)?;
        let message = match owner {
            Some(account) => sign_up_messages::account_already_exists(
                &account.user.email,
                &account.user.name,
                &self.app_url,
            ),
            None => {
                email_change_messages::confirmation(new_email, &user.name, &self.app_url, &token)
            }
        };
        if let Err(error) = self.sender.send(message).await {
            tracing::error!("e-mail change message was not delivered: {error}");
        }
        Ok(())
    }
}
