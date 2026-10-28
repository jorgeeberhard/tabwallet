use chrono::NaiveDate;

use super::{RecurringError, RecurringView};
use crate::{ids::UserId, repositories::recurring::RecurringRepository};

/// Lists the user's recurring Transactions, ended ones included, oldest
/// first.
pub struct ListRecurringTransactionsUseCase {
    recurring: RecurringRepository,
}

impl ListRecurringTransactionsUseCase {
    pub fn new(recurring: RecurringRepository) -> Self {
        Self { recurring }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
    ) -> Result<Vec<RecurringView>, RecurringError> {
        let all = self
            .recurring
            .list(user_id)
            .await
            .map_err(RecurringError::internal)?;
        Ok(all
            .into_iter()
            .map(|recurring| RecurringView::new(recurring, today))
            .collect())
    }
}
