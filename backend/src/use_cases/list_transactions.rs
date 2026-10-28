use std::{cmp::Reverse, collections::HashMap};

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::{
    ids::{CardPurchaseId, CategoryId, RecurringTransactionId, TransactionId, UserId},
    ledger::{Ledger, LedgerFacts, Occurrence, projection_horizon},
    repositories::{
        card_purchases::CardPurchaseRepository,
        credit_cards::CreditCardRepository,
        recurring::{RecurringRepository, RecurringTransaction},
        statement::{StatementKey, StatementRepository},
        transactions::{Transaction, TransactionFilter, TransactionRepository, TransactionType},
    },
};

use super::{
    TransactionError,
    card_purchase::CardPurchaseView,
    credit_card::{card_ledger, view_of},
};

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

/// One entry of the statement: a Transaction outside the credit card, a card
/// purchase (with its card and invoice) or an Occurrence of a recurring
/// Transaction. Invoice payments join as a new variant, with its own data,
/// instead of being squeezed into `Transaction`.
#[derive(Clone, Debug)]
pub enum StatementItem {
    Transaction(Transaction),
    CardPurchase(CardPurchaseView),
    Occurrence(StatementOccurrence),
}

impl StatementItem {
    /// The order of the statement, newest first: date, then creation, then id
    /// (for an Occurrence, the id of its recurring Transaction). It is the
    /// order the database uses for Transactions and card purchases, so all can
    /// be merged.
    fn sort_key(&self) -> (NaiveDate, DateTime<Utc>, Uuid) {
        match self {
            Self::Transaction(transaction) => {
                (transaction.date, transaction.created_at, transaction.id.0)
            }
            Self::CardPurchase(view) => (
                view.purchase.date,
                view.purchase.created_at,
                view.purchase.id.0,
            ),
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

/// UC08: lists the user's Transactions, card purchases and the Occurrences of
/// their recurring Transactions, newest first, optionally narrowed by period,
/// Category, type and credit card.
///
/// Transactions and card purchases are ordered and paged by the database
/// (`StatementRepository`). Occurrences are computed here from the ledger,
/// never read from the database, and merged in. Without an end date the
/// statement reaches the projection horizon (the end of the month 12 months
/// from now); a later `to` is cut there. A credit card filter leaves the
/// Occurrences out: they never belong to a card.
pub struct ListTransactionsUseCase {
    statement: StatementRepository,
    transactions: TransactionRepository,
    purchases: CardPurchaseRepository,
    cards: CreditCardRepository,
    recurring: RecurringRepository,
}

impl ListTransactionsUseCase {
    pub fn new(
        statement: StatementRepository,
        transactions: TransactionRepository,
        purchases: CardPurchaseRepository,
        cards: CreditCardRepository,
        recurring: RecurringRepository,
    ) -> Self {
        Self {
            statement,
            transactions,
            purchases,
            cards,
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
        let occurrences = if filter.credit_card_id.is_some() {
            Vec::new()
        } else {
            self.occurrences(user_id, today, filter).await?
        };
        let total = self
            .statement
            .count(user_id, filter)
            .await
            .map_err(TransactionError::internal)?;

        if occurrences.is_empty() {
            return Ok(Statement {
                items: self.stored(user_id, today, filter, limit, offset).await?,
                total,
            });
        }

        // The page is the `offset..offset + limit` slice of both sorted lists
        // merged, so the first `offset + limit` of each list are enough.
        let occurrence_count = occurrences.len() as i64;
        let head = (offset + limit).max(0);
        let mut items = self.stored(user_id, today, filter, head, 0).await?;
        items.extend(occurrences.into_iter().map(StatementItem::Occurrence));
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

    /// One page of what is stored (Transactions and card purchases), in the
    /// order of the statement.
    async fn stored(
        &self,
        user_id: UserId,
        today: NaiveDate,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StatementItem>, TransactionError> {
        let keys = self
            .statement
            .page(user_id, filter, limit, offset)
            .await
            .map_err(TransactionError::internal)?;

        let transaction_ids: Vec<TransactionId> = keys
            .iter()
            .filter_map(|key| match key {
                StatementKey::Transaction(id) => Some(*id),
                _ => None,
            })
            .collect();
        let purchase_ids: Vec<CardPurchaseId> = keys
            .iter()
            .filter_map(|key| match key {
                StatementKey::CardPurchase(id) => Some(*id),
                _ => None,
            })
            .collect();

        let mut transactions: HashMap<TransactionId, Transaction> = self
            .transactions
            .find_many(user_id, &transaction_ids)
            .await
            .map_err(TransactionError::internal)?
            .into_iter()
            .map(|transaction| (transaction.id, transaction))
            .collect();
        let mut purchases: HashMap<CardPurchaseId, CardPurchaseView> = HashMap::new();
        if !purchase_ids.is_empty() {
            let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id)
                .await
                .map_err(TransactionError::internal)?;
            for purchase in self
                .purchases
                .find_many(user_id, &purchase_ids)
                .await
                .map_err(TransactionError::internal)?
            {
                if let Some(view) = view_of(&ledger, purchase, today) {
                    purchases.insert(view.purchase.id, view);
                }
            }
        }

        Ok(keys
            .into_iter()
            .filter_map(|key| match key {
                StatementKey::Transaction(id) => {
                    transactions.remove(&id).map(StatementItem::Transaction)
                }
                StatementKey::CardPurchase(id) => {
                    purchases.remove(&id).map(StatementItem::CardPurchase)
                }
            })
            .collect())
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
