use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::ids::UserId;

/// A live Pending e-mail change, as the profile shows it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PendingEmailChange {
    /// The new address, as typed.
    pub new_email: String,
    pub expires_at: DateTime<Utc>,
}

/// What changed when a link was confirmed: for the notice to the old address.
#[derive(Clone, Debug)]
pub struct ConfirmedEmailChange {
    pub old_email: String,
    pub new_email: String,
    pub name: String,
}

/// Why a Pending e-mail change could not be confirmed.
#[derive(Debug, thiserror::Error)]
pub enum ConfirmError {
    #[error("the confirmation link is unknown, expired, used or replaced")]
    InvalidToken,
    #[error("the new e-mail already belongs to an account")]
    EmailTaken,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Pending e-mail changes: requests to change the e-mail of an account that
/// wait for the new address to be confirmed. They live outside `users`
/// (ADR-0004), so the account keeps its e-mail until the link is opened.
pub struct PendingEmailChangeRepository {
    pool: PgPool,
}

impl PendingEmailChangeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Deletes the Pending e-mail changes that expired at `now`. Called when a
    /// new request arrives, instead of a scheduled job.
    pub async fn delete_expired(&self, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM pending_email_changes WHERE expires_at <= $1")
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// The account's Pending e-mail change that is still live at `now`.
    pub async fn find_live(
        &self,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<Option<PendingEmailChange>, sqlx::Error> {
        sqlx::query_as(
            "SELECT new_email::text, expires_at FROM pending_email_changes
             WHERE user_id = $1 AND expires_at > $2",
        )
        .bind(user_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
    }

    /// Stores the account's Pending e-mail change, replacing the one it had:
    /// the new address and link win and the old link stops working.
    pub async fn replace(
        &self,
        user_id: UserId,
        new_email: &str,
        token_hash: &str,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO pending_email_changes
                 (user_id, new_email, token_hash, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id) DO UPDATE SET
                 new_email = EXCLUDED.new_email,
                 token_hash = EXCLUDED.token_hash,
                 created_at = EXCLUDED.created_at,
                 expires_at = EXCLUDED.expires_at",
        )
        .bind(user_id)
        .bind(new_email)
        .bind(token_hash)
        .bind(created_at)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Gives the account's live Pending e-mail change a new link, keeping its
    /// address, and returns the address. `None` when there is none.
    pub async fn renew_link(
        &self,
        user_id: UserId,
        token_hash: &str,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            "UPDATE pending_email_changes
             SET token_hash = $2, created_at = $3, expires_at = $4
             WHERE user_id = $1 AND expires_at > $3
             RETURNING new_email::text",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(now)
        .bind(expires_at)
        .fetch_optional(&self.pool)
        .await
    }

    /// Confirms the Pending e-mail change the link belongs to: in one
    /// transaction it deletes it and gives the account the new e-mail.
    /// Claiming is a single `DELETE ... RETURNING`, so two simultaneous
    /// requests with the same link cannot both succeed. The e-mail is unique
    /// in `users` (the database enforces it), so when the new address became
    /// someone's account meanwhile nothing changes and the Pending e-mail
    /// change stays until it expires.
    pub async fn confirm(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<ConfirmedEmailChange, ConfirmError> {
        let mut tx = self.pool.begin().await?;
        let claimed: Option<(UserId, String)> = sqlx::query_as(
            "DELETE FROM pending_email_changes
             WHERE token_hash = $1 AND expires_at > $2
             RETURNING user_id, new_email::text",
        )
        .bind(token_hash)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((user_id, new_email)) = claimed else {
            return Err(ConfirmError::InvalidToken);
        };
        let (old_email, name): (String, String) =
            sqlx::query_as("SELECT email::text, name FROM users WHERE id = $1 FOR UPDATE")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;
        sqlx::query("UPDATE users SET email = $2 WHERE id = $1")
            .bind(user_id)
            .bind(&new_email)
            .execute(&mut *tx)
            .await
            .map_err(|error| match error {
                sqlx::Error::Database(ref db) if db.is_unique_violation() => {
                    ConfirmError::EmailTaken
                }
                other => ConfirmError::Database(other),
            })?;
        tx.commit().await?;
        Ok(ConfirmedEmailChange {
            old_email,
            new_email,
            name,
        })
    }
}
