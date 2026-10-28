use std::collections::HashMap;

use chrono::NaiveDate;

use crate::{
    ids::{CardPurchaseId, TransactionId, UserId},
    repositories::{
        card_purchases::CardPurchaseRepository,
        credit_cards::CreditCardRepository,
        statement::{StatementKey, StatementRepository},
        transactions::{Transaction, TransactionFilter, TransactionRepository},
    },
};

use super::{
    TransactionError,
    card_purchase::CardPurchaseView,
    credit_card::{card_ledger, view_of},
};

/// One entry of the statement: a Transaction outside the credit card, or a
/// card purchase with the card and invoice it belongs to. Invoice payments
/// join as new variants, each with its own data, instead of being squeezed
/// into `Transaction`.
#[derive(Clone, Debug)]
pub enum StatementItem {
    Transaction(Transaction),
    CardPurchase(CardPurchaseView),
}

/// One page of the statement and how many items match in all.
pub struct Statement {
    pub items: Vec<StatementItem>,
    pub total: i64,
}

/// UC08: lists the user's transactions and card purchases, newest first,
/// optionally narrowed by period, Category, type and credit card.
pub struct ListTransactionsUseCase {
    statement: StatementRepository,
    transactions: TransactionRepository,
    purchases: CardPurchaseRepository,
    cards: CreditCardRepository,
}

impl ListTransactionsUseCase {
    pub fn new(
        statement: StatementRepository,
        transactions: TransactionRepository,
        purchases: CardPurchaseRepository,
        cards: CreditCardRepository,
    ) -> Self {
        Self {
            statement,
            transactions,
            purchases,
            cards,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
        today: NaiveDate,
    ) -> Result<Statement, TransactionError> {
        let keys = self
            .statement
            .page(user_id, filter, limit, offset)
            .await
            .map_err(TransactionError::internal)?;
        let total = self
            .statement
            .count(user_id, filter)
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

        let items = keys
            .into_iter()
            .filter_map(|key| match key {
                StatementKey::Transaction(id) => {
                    transactions.remove(&id).map(StatementItem::Transaction)
                }
                StatementKey::CardPurchase(id) => {
                    purchases.remove(&id).map(StatementItem::CardPurchase)
                }
            })
            .collect();
        Ok(Statement { items, total })
    }
}
