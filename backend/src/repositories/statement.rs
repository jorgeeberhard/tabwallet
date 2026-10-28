use sqlx::PgPool;
use uuid::Uuid;

use super::transactions::{TransactionFilter, TransactionType};
use crate::ids::{CardPurchaseId, TransactionId, UserId};

/// Which table a statement entry lives in, with its id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementKey {
    Transaction(TransactionId),
    CardPurchase(CardPurchaseId),
}

#[derive(sqlx::FromRow)]
struct KeyRow {
    kind: String,
    id: Uuid,
}

/// The ordered list of everything that appears in the statement, across the
/// tables that hold it. It only decides which entries make a page and in
/// what order; each entry is then loaded from its own repository.
///
/// A new kind of entry joins by adding a branch to the `UNION ALL` below.
pub struct StatementRepository {
    pool: PgPool,
}

/// The filtered, unioned entries; `$1` is the user and `$2` to `$7` the
/// filter (from, to, Category, type, card).
macro_rules! entries {
    () => {
        "SELECT 'transaction' AS kind, t.id, t.date, t.created_at
         FROM transactions t
         WHERE t.user_id = $1
           AND ($2::date IS NULL OR t.date >= $2)
           AND ($3::date IS NULL OR t.date <= $3)
           AND ($4::uuid IS NULL OR t.category_id = $4)
           AND ($5::text IS NULL OR t.type = $5)
           AND $6::uuid IS NULL
         UNION ALL
         SELECT 'card_purchase' AS kind, p.id, p.date, p.created_at
         FROM card_purchases p
         WHERE p.user_id = $1
           AND ($2::date IS NULL OR p.date >= $2)
           AND ($3::date IS NULL OR p.date <= $3)
           AND ($4::uuid IS NULL OR p.category_id = $4)
           AND ($5::text IS NULL OR p.type = $5)
           AND ($6::uuid IS NULL OR p.credit_card_id = $6)"
    };
}

impl StatementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// One page of entries, newest first. The order is total (date, then
    /// creation, then id), so pages never overlap or skip.
    pub async fn page(
        &self,
        user_id: UserId,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StatementKey>, sqlx::Error> {
        let rows = sqlx::query_as::<_, KeyRow>(concat!(
            "SELECT kind, id FROM (",
            entries!(),
            ") e ORDER BY e.date DESC, e.created_at DESC, e.id DESC LIMIT $7 OFFSET $8"
        ))
        .bind(user_id)
        .bind(filter.from)
        .bind(filter.to)
        .bind(filter.category_id)
        .bind(filter.kind.map(TransactionType::as_str))
        .bind(filter.credit_card_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| match row.kind.as_str() {
                "card_purchase" => StatementKey::CardPurchase(CardPurchaseId(row.id)),
                _ => StatementKey::Transaction(TransactionId(row.id)),
            })
            .collect())
    }

    /// How many entries match the filter, across all pages.
    pub async fn count(
        &self,
        user_id: UserId,
        filter: TransactionFilter,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(concat!("SELECT count(*) FROM (", entries!(), ") e"))
            .bind(user_id)
            .bind(filter.from)
            .bind(filter.to)
            .bind(filter.category_id)
            .bind(filter.kind.map(TransactionType::as_str))
            .bind(filter.credit_card_id)
            .fetch_one(&self.pool)
            .await
    }
}
