use crate::{
    ids::UserId,
    repositories::users::{User, UserRepository},
};

use super::AuthError;

/// Looks up the signed-in user, e.g. so the front-end can show who is signed
/// in and check that a session is still good.
pub struct GetCurrentUserUseCase {
    users: UserRepository,
}

impl GetCurrentUserUseCase {
    pub fn new(users: UserRepository) -> Self {
        Self { users }
    }

    /// A valid token for a user that no longer exists is not a session.
    pub async fn execute(&self, user_id: UserId) -> Result<User, AuthError> {
        self.users
            .find_by_id(user_id)
            .await
            .map_err(AuthError::internal)?
            .ok_or(AuthError::Unauthenticated)
    }
}
