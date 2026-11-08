use chrono::NaiveDate;

use super::{RecurringError, RecurringView, find};
use crate::{
    ids::{CategoryId, RecurringTransactionId, UserId},
    ledger::first_of_month,
    repositories::{categories::CategoryRepository, recurring::RecurringRepository},
    use_cases::transaction_error::ensure_usable_category,
};

/// What an edit may change; `None` leaves the field as it is. The type and the
/// day of the month never change.
#[derive(Default)]
pub struct RecurringChanges {
    pub amount_cents: Option<i64>,
    pub description: Option<String>,
    pub category_id: Option<CategoryId>,
}

/// UC05/UC06: edits a recurring Transaction.
///
/// A new amount applies from the month of the change on (the current month,
/// or the start month if it has not started) and never alters earlier months.
/// The description and the Category are not money, so they apply to every
/// Occurrence. An ended recurring Transaction is history and cannot change.
pub struct UpdateRecurringTransactionUseCase {
    recurring: RecurringRepository,
    categories: CategoryRepository,
}

impl UpdateRecurringTransactionUseCase {
    pub fn new(recurring: RecurringRepository, categories: CategoryRepository) -> Self {
        Self {
            recurring,
            categories,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
        today: NaiveDate,
        changes: RecurringChanges,
    ) -> Result<RecurringView, RecurringError> {
        let current = find(&self.recurring, user_id, id).await?;
        if current.definition.end_month.is_some() {
            return Err(RecurringError::Ended);
        }
        // Keeping the current Category is always allowed, even after it was
        // deactivated: only choosing another one is checked.
        if let Some(category_id) = changes.category_id
            && category_id != current.category_id
        {
            ensure_usable_category(&self.categories, user_id, category_id, current.kind).await?;
        }
        self.recurring
            .update_details(user_id, id, changes.description, changes.category_id)
            .await
            .map_err(RecurringError::internal)?;
        if let Some(amount_cents) = changes.amount_cents {
            let from_month = first_of_month(today).max(current.definition.start_month);
            self.recurring
                .set_amount_from(user_id, id, from_month, amount_cents)
                .await
                .map_err(RecurringError::internal)?;
        }
        let updated = find(&self.recurring, user_id, id).await?;
        Ok(RecurringView::new(updated, today))
    }
}
