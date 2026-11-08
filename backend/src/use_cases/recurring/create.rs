use chrono::NaiveDate;

use super::{RecurringError, RecurringView, find};
use crate::{
    ids::{CategoryId, UserId},
    ledger::first_of_month,
    repositories::{
        categories::CategoryRepository,
        recurring::{NewRecurringTransaction, RecurringRepository},
        transactions::TransactionType,
    },
    use_cases::transaction_error::ensure_usable_category,
};

/// What the user typed to create a recurring Transaction.
pub struct NewRecurring {
    pub kind: TransactionType,
    pub amount_cents: i64,
    pub day_of_month: u32,
    pub description: String,
    pub category_id: CategoryId,
    /// Any day of the first month with an Occurrence; `None` means the month
    /// of today.
    pub start_month: Option<NaiveDate>,
}

/// UC05: registers a Gasto fixo or a Receita fixa. The Category must belong
/// to the user, be active and have the same type. The first Occurrence is in
/// the month of creation (or the month the user picks, never an earlier one).
/// Nothing but the definition is stored: the Occurrences show up as the dates
/// arrive.
pub struct CreateRecurringTransactionUseCase {
    recurring: RecurringRepository,
    categories: CategoryRepository,
}

impl CreateRecurringTransactionUseCase {
    pub fn new(recurring: RecurringRepository, categories: CategoryRepository) -> Self {
        Self {
            recurring,
            categories,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
        new: NewRecurring,
    ) -> Result<RecurringView, RecurringError> {
        let this_month = first_of_month(today);
        let start_month = new.start_month.map_or(this_month, first_of_month);
        if start_month < this_month {
            return Err(RecurringError::Invalid {
                field: "start_month",
                rule: "start_month_past",
            });
        }
        ensure_usable_category(&self.categories, user_id, new.category_id, new.kind).await?;
        let id = self
            .recurring
            .insert(
                user_id,
                NewRecurringTransaction {
                    kind: new.kind,
                    amount_cents: new.amount_cents,
                    day_of_month: new.day_of_month,
                    description: new.description,
                    category_id: new.category_id,
                    start_month,
                },
            )
            .await
            .map_err(RecurringError::internal)?;
        let created = find(&self.recurring, user_id, id).await?;
        Ok(RecurringView::new(created, today))
    }
}
