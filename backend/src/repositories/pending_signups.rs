use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::users::{self, InsertUserError, User};

/// Why a Pending signup could not be confirmed.
#[derive(Debug, thiserror::Error)]
pub enum ConfirmError {
    #[error("the confirmation link is unknown, expired or already used")]
    InvalidToken,
    #[error("the e-mail already belongs to an account")]
    EmailTaken,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Pending signups: sign-up requests whose e-mail is not confirmed yet. They
/// live outside `users` (ADR-0004), so nothing else ever sees them.
pub struct PendingSignupRepository {
    pool: PgPool,
}

impl PendingSignupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Deletes the Pending signups that expired at `now`. Called when a new
    /// request arrives, instead of a scheduled job.
    pub async fn delete_expired(&self, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM pending_signups WHERE expires_at <= $1")
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Stores the Pending signup of an e-mail, replacing the one it had (any
    /// letter case): the new name, password and link win and the old link
    /// stops working.
    pub async fn replace(
        &self,
        email: &str,
        name: &str,
        password_hash: &str,
        token_hash: &str,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO pending_signups
                 (email, name, password_hash, token_hash, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (email) DO UPDATE SET
                 name = EXCLUDED.name,
                 password_hash = EXCLUDED.password_hash,
                 token_hash = EXCLUDED.token_hash,
                 created_at = EXCLUDED.created_at,
                 expires_at = EXCLUDED.expires_at",
        )
        .bind(email)
        .bind(name)
        .bind(password_hash)
        .bind(token_hash)
        .bind(created_at)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Gives the live Pending signup of an e-mail a new link, keeping its
    /// password and name, and returns the e-mail as stored and the name (for
    /// the message). `None` when there is no live Pending signup.
    pub async fn renew_link(
        &self,
        email: &str,
        token_hash: &str,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<Option<(String, String)>, sqlx::Error> {
        sqlx::query_as(
            "UPDATE pending_signups
             SET token_hash = $2, created_at = $3, expires_at = $4
             WHERE email = $1::citext AND expires_at > $3
             RETURNING email::text, name",
        )
        .bind(email)
        .bind(token_hash)
        .bind(now)
        .bind(expires_at)
        .fetch_optional(&self.pool)
        .await
    }

    /// The password hash of the live Pending signup of an e-mail, for the
    /// login to tell "confirm your e-mail" from a wrong password.
    pub async fn find_password_hash(
        &self,
        email: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT password_hash FROM pending_signups
             WHERE email = $1::citext AND expires_at > $2",
        )
        .bind(email)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
    }

    /// Confirms the Pending signup the link belongs to: in one transaction it
    /// deletes the Pending signup and creates the account with its default
    /// categories. Claiming is a single `DELETE ... RETURNING`, so two
    /// simultaneous requests with the same link cannot both succeed. When the
    /// e-mail became an account meanwhile, nothing changes and the Pending
    /// signup stays until it expires.
    pub async fn confirm(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<User, ConfirmError> {
        let mut tx = self.pool.begin().await?;
        let claimed: Option<(String, String, String)> = sqlx::query_as(
            "DELETE FROM pending_signups
             WHERE token_hash = $1 AND expires_at > $2
             RETURNING email::text, name, password_hash",
        )
        .bind(token_hash)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((email, name, password_hash)) = claimed else {
            return Err(ConfirmError::InvalidToken);
        };
        let user = users::insert_with_default_categories(&mut tx, &email, &name, &password_hash)
            .await
            .map_err(|error| match error {
                InsertUserError::EmailTaken => ConfirmError::EmailTaken,
                InsertUserError::Database(error) => ConfirmError::Database(error),
            })?;
        tx.commit().await?;
        Ok(user)
    }
}
