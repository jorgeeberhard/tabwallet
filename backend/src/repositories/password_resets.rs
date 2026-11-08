use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::ids::UserId;

pub struct PasswordResetRepository {
    pool: PgPool,
}

impl PasswordResetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Stores the hash of a new token for the user, replacing any link the
    /// user had asked for before: only the latest e-mail works.
    pub async fn replace_token(
        &self,
        user_id: UserId,
        token_hash: &str,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO password_reset_tokens (user_id, token_hash, created_at, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(created_at)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// Uses the token to set the user's password, if the token exists, has
    /// not been used and has not expired at `now`. Returns whether it did.
    ///
    /// Claiming the token and changing the password happen in one
    /// transaction, and claiming is a single conditional `UPDATE`, so two
    /// simultaneous requests with the same link cannot both succeed. Any
    /// other pending link of the user is dropped.
    pub async fn reset_password(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
        new_password_hash: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let claimed: Option<UserId> = sqlx::query_scalar(
            "UPDATE password_reset_tokens SET used_at = $2
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > $2
             RETURNING user_id",
        )
        .bind(token_hash)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(user_id) = claimed else {
            return Ok(false);
        };
        sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
            .bind(user_id)
            .bind(new_password_hash)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1 AND used_at IS NULL")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }
}
