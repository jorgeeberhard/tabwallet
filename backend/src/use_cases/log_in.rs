use std::sync::Arc;

use crate::{
    auth::{self, TokenService},
    clock::Clock,
    repositories::{pending_signups::PendingSignupRepository, users::UserRepository},
};

use super::{AuthError, Session};

/// UC02: checks the credentials and starts a session.
pub struct LogInUseCase {
    users: UserRepository,
    pending: PendingSignupRepository,
    tokens: TokenService,
    clock: Arc<dyn Clock>,
}

impl LogInUseCase {
    pub fn new(
        users: UserRepository,
        pending: PendingSignupRepository,
        tokens: TokenService,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            pending,
            tokens,
            clock,
        }
    }

    /// Only accounts log in. Fails with the same error for an unknown e-mail
    /// and a wrong password, and takes as long in both cases (one password
    /// check against a real or decoy hash), so the response does not reveal
    /// which e-mails have an account.
    ///
    /// The one exception: when there is no account but the e-mail and the
    /// password both match a live Pending signup, it fails with
    /// `EmailNotConfirmed`, so the person learns what is missing. Anyone who
    /// can see that holds the password already, so it reveals nothing to a
    /// stranger.
    pub async fn execute(&self, email: &str, password: &str) -> Result<Session, AuthError> {
        let found = self
            .users
            .find_credentials_by_email(email)
            .await
            .map_err(AuthError::internal)?;
        let (stored_hash, account, is_pending) = match found {
            Some(credentials) => (credentials.password_hash, Some(credentials.user), false),
            None => {
                let pending_hash = self
                    .pending
                    .find_password_hash(email, self.clock.now())
                    .await
                    .map_err(AuthError::internal)?;
                match pending_hash {
                    Some(hash) => (hash, None, true),
                    None => (auth::password::decoy_hash(), None, false),
                }
            }
        };
        let password_matches = auth::password::verify(password.to_string(), stored_hash).await;
        if is_pending && password_matches {
            return Err(AuthError::EmailNotConfirmed);
        }
        let user = account
            .filter(|_| password_matches)
            .ok_or(AuthError::InvalidCredentials)?;
        let token = self.tokens.issue(user.id).map_err(AuthError::internal)?;
        Ok(Session { user, token })
    }
}
