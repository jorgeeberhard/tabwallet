use crate::{
    ids::{TransactionId, UserId},
    repositories::{
        categories::CategoryRepository,
        transactions::{Transaction, TransactionChanges, TransactionRepository},
    },
};

use super::{TransactionError, transaction_error::ensure_usable_category};

/// UC06: edits amount, date, description and Category of a Transaction. The
/// type never changes, so a new Category must have the type the Transaction
/// already has. Keeping the current Category is always allowed, even after it
/// was deactivated: only choosing another one is checked.
pub struct UpdateTransactionUseCase {
    transactions: TransactionRepository,
    categories: CategoryRepository,
}

impl UpdateTransactionUseCase {
    pub fn new(transactions: TransactionRepository, categories: CategoryRepository) -> Self {
        Self {
            transactions,
            categories,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: TransactionId,
        changes: TransactionChanges,
    ) -> Result<Transaction, TransactionError> {
        let current = self
            .transactions
            .find(user_id, id)
            .await
            .map_err(TransactionError::internal)?
            .ok_or(TransactionError::NotFound)?;
        if let Some(category_id) = changes.category_id
            && category_id != current.category_id
        {
            ensure_usable_category(&self.categories, user_id, category_id, current.kind).await?;
        }
        self.transactions
            .update(user_id, id, changes)
            .await
            .map_err(TransactionError::internal)?
            .ok_or(TransactionError::NotFound)
    }
}
