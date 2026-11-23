use crate::{
    ids::UserId,
    repositories::{
        categories::CategoryRepository,
        transactions::{NewTransaction, Transaction, TransactionRepository},
    },
};

use super::{TransactionError, transaction_error::ensure_usable_category};

/// UC04: records an income or an expense outside the credit card. The
/// Category must belong to the user, be active and have the same type. A date
/// after today makes it a Projection, which needs nothing special here.
pub struct CreateTransactionUseCase {
    transactions: TransactionRepository,
    categories: CategoryRepository,
}

impl CreateTransactionUseCase {
    pub fn new(transactions: TransactionRepository, categories: CategoryRepository) -> Self {
        Self {
            transactions,
            categories,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        new: NewTransaction,
    ) -> Result<Transaction, TransactionError> {
        ensure_usable_category(&self.categories, user_id, new.category_id, new.kind).await?;
        self.transactions
            .insert(user_id, new)
            .await
            .map_err(TransactionError::internal)
    }
}
