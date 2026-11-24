use std::sync::Arc;

use crate::{
    clock::Clock,
    ids::UserId,
    repositories::{pending_email_changes::PendingEmailChangeRepository, users::UserRepository},
};

use super::{AuthError, ProfileView};

/// Looks up the signed-in user, so the front-end can show who is signed
/// in and check that a session is still good, together with the e-mail change
/// that waits for confirmation, if any.
pub struct GetCurrentUserUseCase {
    users: UserRepository,
    pending: PendingEmailChangeRepository,
    clock: Arc<dyn Clock>,
}

impl GetCurrentUserUseCase {
    pub fn new(
        users: UserRepository,
        pending: PendingEmailChangeRepository,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            pending,
            clock,
        }
    }

    /// A valid token for a user that no longer exists is not a session. An
    /// expired Pending e-mail change is not shown.
    pub async fn execute(&self, user_id: UserId) -> Result<ProfileView, AuthError> {
        let user = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(AuthError::internal)?
            .ok_or(AuthError::Unauthenticated)?;
        let pending_email_change = self
            .pending
            .find_live(user_id, self.clock.now())
            .await
            .map_err(AuthError::internal)?;
        Ok(ProfileView {
            user,
            pending_email_change,
        })
    }
}
