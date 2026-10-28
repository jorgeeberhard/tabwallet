use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use super::categories::CategoryType;
use crate::ids::{CategoryId, TransactionId, UserId};

/// A Transaction is an income or an expense, like its Category.
pub type TransactionType = CategoryType;

/// A Transaction outside the credit card, with the name and state of its
/// Category (a deactivated Category stays on the transactions that use it).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Transaction {
    pub id: TransactionId,
    #[sqlx(rename = "type", try_from = "String")]
    pub kind: TransactionType,
    /// Always greater than zero; `kind` gives the direction.
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
    pub category_name: String,
    pub category_active: bool,
    pub created_at: DateTime<Utc>,
}

/// What a new Transaction is made of. The caller has already checked the
/// Category (it belongs to the user, is active and has the same type).
pub struct NewTransaction {
    pub kind: TransactionType,
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
}

/// What an edit may change; `None` leaves the field as it is. The type never
/// changes. The caller has already checked a new Category.
#[derive(Default)]
pub struct TransactionChanges {
    pub amount_cents: Option<i64>,
    pub date: Option<NaiveDate>,
    pub description: Option<String>,
    pub category_id: Option<CategoryId>,
}

/// The columns of [`Transaction`]; `t` is the transactions table (or a CTE
/// with its columns) and the Category is joined as `c`.
///
/// A macro, not a `const`, because `sqlx` only takes literal SQL and `concat!`
/// needs a literal.
macro_rules! columns {
    () => {
        "t.id, t.type, t.amount_cents, t.date, t.description, t.category_id,
         c.name AS category_name, c.active AS category_active, t.created_at"
    };
}

pub struct TransactionRepository {
    pool: PgPool,
}

impl TransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        user_id: UserId,
        new: NewTransaction,
    ) -> Result<Transaction, sqlx::Error> {
        sqlx::query_as::<_, Transaction>(concat!(
            "WITH t AS (
                 INSERT INTO transactions (user_id, type, amount_cents, date, description, category_id)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING *
             )
             SELECT ",
            columns!(),
            " FROM t JOIN categories c ON c.id = t.category_id"
        ))
        .bind(user_id)
        .bind(new.kind.as_str())
        .bind(new.amount_cents)
        .bind(new.date)
        .bind(new.description)
        .bind(new.category_id)
        .fetch_one(&self.pool)
        .await
    }

    /// One page of the user's transactions, newest first. The order is total
    /// (date, then creation, then id), so pages never overlap or skip.
    pub async fn list(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>, sqlx::Error> {
        sqlx::query_as::<_, Transaction>(concat!(
            "SELECT ",
            columns!(),
            " FROM transactions t JOIN categories c ON c.id = t.category_id
             WHERE t.user_id = $1
             ORDER BY t.date DESC, t.created_at DESC, t.id DESC
             LIMIT $2 OFFSET $3"
        ))
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// How many transactions the user has.
    pub async fn count(&self, user_id: UserId) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT count(*) FROM transactions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
    }

    /// One of the user's transactions, or `None` if there is none with this id
    /// (including one that belongs to someone else).
    pub async fn find(
        &self,
        user_id: UserId,
        id: TransactionId,
    ) -> Result<Option<Transaction>, sqlx::Error> {
        sqlx::query_as::<_, Transaction>(concat!(
            "SELECT ",
            columns!(),
            " FROM transactions t JOIN categories c ON c.id = t.category_id
             WHERE t.user_id = $1 AND t.id = $2"
        ))
        .bind(user_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Applies the changes to one of the user's transactions; `None` if it
    /// does not exist.
    pub async fn update(
        &self,
        user_id: UserId,
        id: TransactionId,
        changes: TransactionChanges,
    ) -> Result<Option<Transaction>, sqlx::Error> {
        sqlx::query_as::<_, Transaction>(concat!(
            "WITH t AS (
                 UPDATE transactions SET
                     amount_cents = COALESCE($3, amount_cents),
                     date = COALESCE($4, date),
                     description = COALESCE($5, description),
                     category_id = COALESCE($6, category_id),
                     updated_at = now()
                 WHERE user_id = $1 AND id = $2
                 RETURNING *
             )
             SELECT ",
            columns!(),
            " FROM t JOIN categories c ON c.id = t.category_id"
        ))
        .bind(user_id)
        .bind(id)
        .bind(changes.amount_cents)
        .bind(changes.date)
        .bind(changes.description)
        .bind(changes.category_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes one of the user's transactions; `false` if it does not exist.
    pub async fn delete(&self, user_id: UserId, id: TransactionId) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM transactions WHERE user_id = $1 AND id = $2")
            .bind(user_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
