use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use super::transactions::TransactionType;
use crate::{
    ids::{CategoryId, RecurringTransactionId, UserId},
    ledger::{AmountFrom, FlowKind, OccurrenceChange, OccurrenceException, RecurringDefinition},
};

/// A recurring Transaction (Gasto fixo or Receita fixa) with the name and
/// state of its Category. The Occurrences are not here: the ledger derives
/// them from `definition`.
#[derive(Clone, Debug)]
pub struct RecurringTransaction {
    pub id: RecurringTransactionId,
    pub kind: TransactionType,
    pub description: String,
    pub category_id: CategoryId,
    pub category_name: String,
    pub category_active: bool,
    pub created_at: DateTime<Utc>,
    /// What the ledger needs: day, months, amount versions and exceptions.
    pub definition: RecurringDefinition,
}

/// What a new recurring Transaction is made of. The caller has already checked
/// the Category (it belongs to the user, is active and has the same type) and
/// the start month.
pub struct NewRecurringTransaction {
    pub kind: TransactionType,
    pub amount_cents: i64,
    pub day_of_month: u32,
    pub description: String,
    pub category_id: CategoryId,
    /// First day of the first month with an Occurrence.
    pub start_month: NaiveDate,
}

#[derive(sqlx::FromRow)]
struct RecurringRow {
    id: RecurringTransactionId,
    #[sqlx(rename = "type", try_from = "String")]
    kind: TransactionType,
    description: String,
    category_id: CategoryId,
    category_name: String,
    category_active: bool,
    day_of_month: i16,
    start_month: NaiveDate,
    end_month: Option<NaiveDate>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct AmountRow {
    recurring_transaction_id: RecurringTransactionId,
    effective_month: NaiveDate,
    amount_cents: i64,
}

#[derive(sqlx::FromRow)]
struct ExceptionRow {
    recurring_transaction_id: RecurringTransactionId,
    month: NaiveDate,
    skipped: bool,
    amount_cents: Option<i64>,
}

pub struct RecurringRepository {
    pool: PgPool,
}

impl RecurringRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Stores a new recurring Transaction with its first amount version, in
    /// one transaction, and returns its id.
    pub async fn insert(
        &self,
        user_id: UserId,
        new: NewRecurringTransaction,
    ) -> Result<RecurringTransactionId, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let id: RecurringTransactionId = sqlx::query_scalar(
            "INSERT INTO recurring_transactions
                 (user_id, type, description, category_id, day_of_month, start_month)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id",
        )
        .bind(user_id)
        .bind(new.kind.as_str())
        .bind(new.description)
        .bind(new.category_id)
        .bind(new.day_of_month as i16)
        .bind(new.start_month)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO recurring_amounts (recurring_transaction_id, effective_month, amount_cents)
             VALUES ($1, $2, $3)",
        )
        .bind(id)
        .bind(new.start_month)
        .bind(new.amount_cents)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Every recurring Transaction of the user, ended ones included, oldest
    /// first.
    pub async fn list(&self, user_id: UserId) -> Result<Vec<RecurringTransaction>, sqlx::Error> {
        self.load(user_id, None).await
    }

    /// One of the user's recurring Transactions, or `None` if there is none
    /// with this id (including one that belongs to someone else).
    pub async fn find(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
    ) -> Result<Option<RecurringTransaction>, sqlx::Error> {
        Ok(self.load(user_id, Some(id)).await?.into_iter().next())
    }

    async fn load(
        &self,
        user_id: UserId,
        id: Option<RecurringTransactionId>,
    ) -> Result<Vec<RecurringTransaction>, sqlx::Error> {
        let rows = sqlx::query_as::<_, RecurringRow>(
            "SELECT r.id, r.type, r.description, r.category_id,
                    c.name AS category_name, c.active AS category_active,
                    r.day_of_month, r.start_month, r.end_month, r.created_at
             FROM recurring_transactions r JOIN categories c ON c.id = r.category_id
             WHERE r.user_id = $1 AND ($2::uuid IS NULL OR r.id = $2)
             ORDER BY r.created_at, r.id",
        )
        .bind(user_id)
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        let amounts = sqlx::query_as::<_, AmountRow>(
            "SELECT a.recurring_transaction_id, a.effective_month, a.amount_cents
             FROM recurring_amounts a
             JOIN recurring_transactions r ON r.id = a.recurring_transaction_id
             WHERE r.user_id = $1 AND ($2::uuid IS NULL OR r.id = $2)",
        )
        .bind(user_id)
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        let exceptions = sqlx::query_as::<_, ExceptionRow>(
            "SELECT e.recurring_transaction_id, e.month, e.skipped, e.amount_cents
             FROM recurring_exceptions e
             JOIN recurring_transactions r ON r.id = e.recurring_transaction_id
             WHERE r.user_id = $1 AND ($2::uuid IS NULL OR r.id = $2)",
        )
        .bind(user_id)
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let mut amounts_by_id: HashMap<RecurringTransactionId, Vec<AmountFrom>> = HashMap::new();
        for row in amounts {
            amounts_by_id
                .entry(row.recurring_transaction_id)
                .or_default()
                .push(AmountFrom {
                    month: row.effective_month,
                    amount_cents: row.amount_cents,
                });
        }
        let mut exceptions_by_id: HashMap<RecurringTransactionId, Vec<OccurrenceException>> =
            HashMap::new();
        for row in exceptions {
            let change = match (row.skipped, row.amount_cents) {
                (false, Some(amount)) => OccurrenceChange::Amount(amount),
                _ => OccurrenceChange::Skipped,
            };
            exceptions_by_id
                .entry(row.recurring_transaction_id)
                .or_default()
                .push(OccurrenceException {
                    month: row.month,
                    change,
                });
        }

        Ok(rows
            .into_iter()
            .map(|row| RecurringTransaction {
                id: row.id,
                kind: row.kind,
                description: row.description,
                category_id: row.category_id,
                category_name: row.category_name,
                category_active: row.category_active,
                created_at: row.created_at,
                definition: RecurringDefinition {
                    id: row.id,
                    kind: match row.kind {
                        TransactionType::Income => FlowKind::Income,
                        TransactionType::Expense => FlowKind::Expense,
                    },
                    day_of_month: row.day_of_month as u32,
                    start_month: row.start_month,
                    end_month: row.end_month,
                    amounts: amounts_by_id.remove(&row.id).unwrap_or_default(),
                    exceptions: exceptions_by_id.remove(&row.id).unwrap_or_default(),
                },
            })
            .collect())
    }

    /// Changes the description and/or the Category, which apply to every
    /// Occurrence. `false` if the recurring Transaction does not exist.
    pub async fn update_details(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
        description: Option<String>,
        category_id: Option<CategoryId>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE recurring_transactions SET
                 description = COALESCE($3, description),
                 category_id = COALESCE($4, category_id),
                 updated_at = now()
             WHERE user_id = $1 AND id = $2",
        )
        .bind(user_id)
        .bind(id)
        .bind(description)
        .bind(category_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Makes `amount_cents` the amount from `month` on, replacing the version
    /// of that same month if there is one. Earlier months keep their amount.
    pub async fn set_amount_from(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
        month: NaiveDate,
        amount_cents: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO recurring_amounts (recurring_transaction_id, effective_month, amount_cents)
             SELECT r.id, $3, $4 FROM recurring_transactions r
             WHERE r.user_id = $1 AND r.id = $2
             ON CONFLICT (recurring_transaction_id, effective_month)
             DO UPDATE SET amount_cents = EXCLUDED.amount_cents",
        )
        .bind(user_id)
        .bind(id)
        .bind(month)
        .bind(amount_cents)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Ends the recurring Transaction: `last_month` is its last month with an
    /// Occurrence.
    pub async fn end(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
        last_month: NaiveDate,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE recurring_transactions SET end_month = $3, updated_at = now()
             WHERE user_id = $1 AND id = $2",
        )
        .bind(user_id)
        .bind(id)
        .bind(last_month)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Records that the user deleted, or edited the amount of, the single
    /// Occurrence of `month`, replacing an earlier exception of that month.
    pub async fn set_exception(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
        month: NaiveDate,
        change: OccurrenceChange,
    ) -> Result<(), sqlx::Error> {
        let (skipped, amount_cents) = match change {
            OccurrenceChange::Skipped => (true, None),
            OccurrenceChange::Amount(amount) => (false, Some(amount)),
        };
        sqlx::query(
            "INSERT INTO recurring_exceptions (recurring_transaction_id, month, skipped, amount_cents)
             SELECT r.id, $3, $4, $5 FROM recurring_transactions r
             WHERE r.user_id = $1 AND r.id = $2
             ON CONFLICT (recurring_transaction_id, month)
             DO UPDATE SET skipped = EXCLUDED.skipped, amount_cents = EXCLUDED.amount_cents",
        )
        .bind(user_id)
        .bind(id)
        .bind(month)
        .bind(skipped)
        .bind(amount_cents)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
