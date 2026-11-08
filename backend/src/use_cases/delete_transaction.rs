use crate::{
    ids::{TransactionId, UserId},
    repositories::transactions::TransactionRepository,
};

use super::TransactionError;

/// UC07: removes a Transaction.
pub struct DeleteTransactionUseCase {
    transactions: TransactionRepository,
}

impl DeleteTransactionUseCase {
    pub fn new(transactions: TransactionRepository) -> Self {
        Self { transactions }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: TransactionId,
    ) -> Result<(), TransactionError> {
        match self.transactions.delete(user_id, id).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(TransactionError::NotFound),
            Err(error) => Err(TransactionError::internal(error)),
        }
    }
}
