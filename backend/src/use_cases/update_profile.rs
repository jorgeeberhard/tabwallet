use crate::{
    ids::UserId,
    repositories::users::{InsertUserError, User, UserRepository},
};

use super::AuthError;

/// What the user asked to change. `None` leaves a field as it is.
pub struct ProfileChanges {
    pub name: Option<String>,
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
            .ok_or(AuthError::Unauthenticated)?
            .user;
        let name = changes.name.unwrap_or(current.name);
        self.users
            .update_profile(user_id, &name, &current.email, None)
            .await
            .map_err(|error| match error {
                InsertUserError::EmailTaken => AuthError::EmailTaken,
                InsertUserError::Database(error) => AuthError::internal(error),
            })
    }
}
