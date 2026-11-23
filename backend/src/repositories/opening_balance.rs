use sqlx::PgPool;

use crate::ids::UserId;

/// Reads and writes the user's Opening Balance (UC22).
pub struct OpeningBalanceRepository {
    pool: PgPool,
}

impl OpeningBalanceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The Opening Balance in cents, or `None` if the user never informed one.
    pub async fn find(&self, user_id: UserId) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar("SELECT opening_balance_cents FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map(Option::flatten)
    }

    /// Records the Opening Balance only if none was informed yet, in one
    /// statement, so two concurrent requests cannot both win. `false` if it
    /// was already set (the stored value is left untouched).
    pub async fn set_once(&self, user_id: UserId, amount_cents: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE users SET opening_balance_cents = $2
             WHERE id = $1 AND opening_balance_cents IS NULL",
        )
        .bind(user_id)
        .bind(amount_cents)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
