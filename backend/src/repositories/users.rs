use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::categories;
use crate::ids::UserId;

/// A user as the rest of the system sees it. Never carries the password hash.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

/// A user together with the stored password hash, for checking a login.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct UserCredentials {
    #[sqlx(flatten)]
    pub user: User,
    pub password_hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum InsertUserError {
    #[error("e-mail already registered")]
    EmailTaken,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: UserId) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT id, email, created_at FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// The user and password hash for an e-mail (any letter case), if any.
    pub async fn find_credentials_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserCredentials>, sqlx::Error> {
        sqlx::query_as::<_, UserCredentials>(
            "SELECT id, email, created_at, password_hash FROM users WHERE email = $1::citext",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    /// Creates the user together with their default categories, in one
    /// transaction: a new account never exists without them. The e-mail is
    /// unique regardless of letter case; the database enforces it, so two
    /// simultaneous sign-ups cannot both win.
    pub async fn insert(&self, email: &str, password_hash: &str) -> Result<User, InsertUserError> {
        let mut transaction = self.pool.begin().await?;
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2)
             RETURNING id, email, created_at",
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(ref db) if db.is_unique_violation() => {
                InsertUserError::EmailTaken
            }
            other => InsertUserError::Database(other),
        })?;
        categories::seed_default_categories(&mut transaction, user.id).await?;
        transaction.commit().await?;
        Ok(user)
    }
}
