use crate::{
    auth::{self, TokenService},
    repositories::users::UserRepository,
};

use super::{AuthError, Session};

/// UC02: checks the credentials and starts a session.
pub struct LogInUseCase {
    users: UserRepository,
    tokens: TokenService,
}

impl LogInUseCase {
    pub fn new(users: UserRepository, tokens: TokenService) -> Self {
        Self { users, tokens }
    }

    /// Fails with the same error for an unknown e-mail and a wrong password,
    /// and takes as long in both cases, so the response does not reveal which
    /// e-mails have an account.
    pub async fn execute(&self, email: &str, password: &str) -> Result<Session, AuthError> {
        let found = self
            .users
            .find_credentials_by_email(email)
            .await
            .map_err(AuthError::internal)?;
        let (stored_hash, account) = match found {
            Some(credentials) => (credentials.password_hash, Some(credentials.user)),
            None => (auth::password::decoy_hash(), None),
        };
        let password_matches = auth::password::verify(password.to_string(), stored_hash).await;
        let user = account
            .filter(|_| password_matches)
            .ok_or(AuthError::InvalidCredentials)?;
        let token = self.tokens.issue(user.id).map_err(AuthError::internal)?;
        Ok(Session { user, token })
    }
}
