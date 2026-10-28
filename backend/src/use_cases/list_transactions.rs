use crate::{
    ids::UserId,
    repositories::transactions::{Transaction, TransactionFilter, TransactionRepository},
};

use super::TransactionError;

/// One entry of the statement. Today only Transactions are in it; credit-card
/// purchases and invoice payments join as new variants, each with its own
/// data, instead of being squeezed into `Transaction`.
#[derive(Clone, Debug)]
pub enum StatementItem {
    Transaction(Transaction),
}

/// One page of the statement and how many items match in all.
pub struct Statement {
    pub items: Vec<StatementItem>,
    pub total: i64,
}

/// UC08: lists the user's transactions, newest first, optionally narrowed by
/// period, Category and type.
pub struct ListTransactionsUseCase {
    transactions: TransactionRepository,
}

impl ListTransactionsUseCase {
    pub fn new(transactions: TransactionRepository) -> Self {
        Self { transactions }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Statement, TransactionError> {
        let transactions = self
            .transactions
            .list(user_id, filter, limit, offset)
            .await
            .map_err(TransactionError::internal)?;
        let total = self
            .transactions
            .count(user_id, filter)
            .await
            .map_err(TransactionError::internal)?;
        Ok(Statement {
            items: transactions
                .into_iter()
                .map(StatementItem::Transaction)
                .collect(),
            total,
        })
    }
}
