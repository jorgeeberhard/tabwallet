use crate::{
    auth,
    ids::UserId,
    repositories::users::{InsertUserError, User, UserRepository},
};

use super::AuthError;

/// What the user asked to change. `None` leaves a field as it is.
pub struct ProfileChanges {
    pub name: Option<String>,
    pub email: Option<String>,
    pub new_password: Option<String>,
    /// Proves who is asking; required to change the e-mail or the password.
    pub current_password: Option<String>,
}

/// UC14: edits the profile.
pub struct UpdateProfileUseCase {
    users: UserRepository,
}

impl UpdateProfileUseCase {
    pub fn new(users: UserRepository) -> Self {
        Self { users }
    }

    /// The fields are already validated by the handler.
    ///
    /// Sessions already open stay valid after an e-mail or password change:
    /// the session token is a JWT, which cannot be revoked (see README).
    pub async fn execute(
        &self,
        user_id: UserId,
        changes: ProfileChanges,
    ) -> Result<User, AuthError> {
        let current = self
            .users
            .find_credentials_by_id(user_id)
            .await
            .map_err(AuthError::internal)?
            .ok_or(AuthError::Unauthenticated)?;

        let email = changes.email.unwrap_or_else(|| current.user.email.clone());
        let is_sensitive = email != current.user.email || changes.new_password.is_some();
        if is_sensitive {
            let given = changes
                .current_password
                .ok_or(AuthError::CurrentPasswordRequired)?;
            if !auth::password::verify(given, current.password_hash.clone()).await {
                return Err(AuthError::InvalidCurrentPassword);
            }
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
        self.users
            .update_profile(user_id, &name, &email, new_password_hash.as_deref())
            .await
            .map_err(|error| match error {
                InsertUserError::EmailTaken => AuthError::EmailTaken,
                InsertUserError::Database(error) => AuthError::internal(error),
            })
    }
}
