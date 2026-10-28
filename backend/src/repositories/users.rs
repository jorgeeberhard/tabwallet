use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::ids::UserId;

/// A user as the rest of the system sees it. Never carries the password hash.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub name: String,
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
        sqlx::query_as::<_, User>("SELECT id, email, name, created_at FROM users WHERE id = $1")
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
            "SELECT id, email, name, created_at, password_hash FROM users WHERE email = $1::citext",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    /// The user and password hash for an id, if any.
    pub async fn find_credentials_by_id(
        &self,
        id: UserId,
    ) -> Result<Option<UserCredentials>, sqlx::Error> {
        sqlx::query_as::<_, UserCredentials>(
            "SELECT id, email, name, created_at, password_hash FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Stores the profile as given. `new_password_hash` replaces the password
    /// when present, and then every pending password-reset link of the user
    /// dies with the old password, in the same transaction.
    pub async fn update_profile(
        &self,
        id: UserId,
        name: &str,
        email: &str,
        new_password_hash: Option<&str>,
    ) -> Result<User, InsertUserError> {
        let mut tx = self.pool.begin().await?;
        let user = sqlx::query_as::<_, User>(
            "UPDATE users
             SET name = $2, email = $3, password_hash = COALESCE($4, password_hash)
             WHERE id = $1
             RETURNING id, email, name, created_at",
        )
        .bind(id)
        .bind(name)
        .bind(email)
        .bind(new_password_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(ref db) if db.is_unique_violation() => {
                InsertUserError::EmailTaken
            }
            other => InsertUserError::Database(other),
        })?;
        if new_password_hash.is_some() {
            sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(user)
    }

    /// Creates the user. The e-mail is unique regardless of letter case; the
    /// database enforces it, so two simultaneous sign-ups cannot both win.
    pub async fn insert(&self, email: &str, password_hash: &str) -> Result<User, InsertUserError> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2)
             RETURNING id, email, name, created_at",
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(ref db) if db.is_unique_violation() => {
                InsertUserError::EmailTaken
            }
            other => InsertUserError::Database(other),
        })
    }
}
