use crate::{
    auth::{
        self, TokenService,
        password_policy::{self, Account},
    },
    repositories::users::{InsertUserError, User, UserRepository},
};

use super::{AuthError, Session};

/// UC01: creates an account and signs the new user in.
pub struct SignUpUseCase {
    users: UserRepository,
    tokens: TokenService,
}

impl SignUpUseCase {
    pub fn new(users: UserRepository, tokens: TokenService) -> Self {
        Self { users, tokens }
    }

    /// `email` is already validated by the handler; the password is judged by
    /// the Password policy against that e-mail (a sign-up has no name yet).
    pub async fn execute(&self, email: &str, password: &str) -> Result<Session, AuthError> {
        password_policy::check(password, Account { email, name: "" })
            .map_err(AuthError::PasswordRejected)?;
        let password_hash = auth::password::hash(password.to_string())
            .await
            .map_err(AuthError::Internal)?;
        let user: User =
            self.users
                .insert(email, &password_hash)
                .await
                .map_err(|error| match error {
                    InsertUserError::EmailTaken => AuthError::EmailTaken,
                    InsertUserError::Database(error) => AuthError::internal(error),
                })?;
        let token = self.tokens.issue(user.id).map_err(AuthError::internal)?;
        Ok(Session { user, token })
    }
}
