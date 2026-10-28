use std::cmp::Reverse;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::{
    ids::{CategoryId, RecurringTransactionId, UserId},
    ledger::{Ledger, LedgerFacts, Occurrence, projection_horizon},
    repositories::{
        recurring::{RecurringRepository, RecurringTransaction},
        transactions::{Transaction, TransactionFilter, TransactionRepository, TransactionType},
    },
};

use super::TransactionError;

/// One Occurrence of a recurring Transaction (a Gasto fixo or Receita fixa) in
/// the statement, with the data of the recurring Transaction it came from. It
/// is derived on every read: it is a Projection while its date is after today.
#[derive(Clone, Debug)]
pub struct StatementOccurrence {
    pub recurring_id: RecurringTransactionId,
    /// First day of its month; with `recurring_id` it identifies the
    /// Occurrence.
    pub month: NaiveDate,
    pub kind: TransactionType,
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
    pub category_name: String,
    pub category_active: bool,
    /// The user edited the amount of this Occurrence only.
    pub edited: bool,
    /// When the recurring Transaction was created; it breaks ties in the
    /// order of the statement.
    pub created_at: DateTime<Utc>,
}

impl StatementOccurrence {
    pub fn new(recurring: &RecurringTransaction, occurrence: Occurrence) -> Self {
        Self {
            recurring_id: recurring.id,
            month: occurrence.month,
            kind: recurring.kind,
            amount_cents: occurrence.amount_cents,
            date: occurrence.date,
            description: recurring.description.clone(),
            category_id: recurring.category_id,
            category_name: recurring.category_name.clone(),
            category_active: recurring.category_active,
            edited: occurrence.edited,
            created_at: recurring.created_at,
        }
    }
}

/// One entry of the statement. Credit-card purchases and invoice payments join
/// as new variants, each with its own data, instead of being squeezed into
/// `Transaction`.
#[derive(Clone, Debug)]
pub enum StatementItem {
    Transaction(Transaction),
    Occurrence(StatementOccurrence),
}

impl StatementItem {
    /// The order of the statement, newest first: date, then creation, then id
    /// (for an Occurrence, the id of its recurring Transaction). It is the
    /// order the database uses for Transactions, so both can be merged.
    fn sort_key(&self) -> (NaiveDate, DateTime<Utc>, Uuid) {
        match self {
            Self::Transaction(transaction) => {
                (transaction.date, transaction.created_at, transaction.id.0)
            }
            Self::Occurrence(occurrence) => (
                occurrence.date,
                occurrence.created_at,
                occurrence.recurring_id.0,
            ),
        }
    }
}

/// One page of the statement and how many items match in all.
pub struct Statement {
    pub items: Vec<StatementItem>,
    pub total: i64,
}

/// UC08: lists the user's Transactions and the Occurrences of their recurring
/// Transactions, newest first, optionally narrowed by period, Category and
/// type.
///
/// Occurrences are computed here from the ledger, never read from the
/// database. Without an end date the statement reaches the projection horizon
/// (the end of the month 12 months from now); a later `to` is cut there.
pub struct ListTransactionsUseCase {
    transactions: TransactionRepository,
    recurring: RecurringRepository,
}

impl ListTransactionsUseCase {
    pub fn new(transactions: TransactionRepository, recurring: RecurringRepository) -> Self {
        Self {
            transactions,
            recurring,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Statement, TransactionError> {
        let occurrences = self.occurrences(user_id, today, filter).await?;
        let total = self
            .transactions
            .count(user_id, filter)
            .await
            .map_err(TransactionError::internal)?;

        if occurrences.is_empty() {
            let transactions = self
                .transactions
                .list(user_id, filter, limit, offset)
                .await
                .map_err(TransactionError::internal)?;
            return Ok(Statement {
                items: transactions
                    .into_iter()
                    .map(StatementItem::Transaction)
                    .collect(),
                total,
            });
        }

        // The page is the `offset..offset + limit` slice of both sorted lists
        // merged, so the first `offset + limit` of each list are enough.
        let occurrence_count = occurrences.len() as i64;
        let head = (offset + limit).max(0);
        let transactions = self
            .transactions
            .list(user_id, filter, head, 0)
            .await
            .map_err(TransactionError::internal)?;
        let mut items: Vec<StatementItem> = transactions
            .into_iter()
            .map(StatementItem::Transaction)
            .chain(occurrences.into_iter().map(StatementItem::Occurrence))
            .collect();
        items.sort_by_key(|item| Reverse(item.sort_key()));
        Ok(Statement {
            total: total + occurrence_count,
            items: items
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect(),
        })
    }

    /// The Occurrences inside the filter's period (up to the projection
    /// horizon) whose Category and type match.
    async fn occurrences(
        &self,
        user_id: UserId,
        today: NaiveDate,
        filter: TransactionFilter,
    ) -> Result<Vec<StatementOccurrence>, TransactionError> {
        let recurring = self
            .recurring
            .list(user_id)
            .await
            .map_err(TransactionError::internal)?;
        if recurring.is_empty() {
            return Ok(Vec::new());
        }
        let from = filter.from.unwrap_or(NaiveDate::MIN);
        let to = filter.to.map_or(projection_horizon(today), |to| {
            to.min(projection_horizon(today))
        });
        let ledger = Ledger::new(LedgerFacts {
            recurring: recurring
                .iter()
                .map(|recurring| recurring.definition.clone())
                .collect(),
            ..LedgerFacts::default()
        });
        Ok(ledger
            .occurrences(from, to)
            .into_iter()
            .filter_map(|occurrence| {
                let source = recurring
                    .iter()
                    .find(|recurring| recurring.id == occurrence.recurring_id)?;
                let kind_matches = filter.kind.is_none_or(|kind| kind == source.kind);
                let category_matches = filter
                    .category_id
                    .is_none_or(|category_id| category_id == source.category_id);
                (kind_matches && category_matches)
                    .then(|| StatementOccurrence::new(source, occurrence))
            })
            .collect())
    }
}
