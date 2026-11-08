use chrono::NaiveDate;

use super::{RecurringError, RecurringView, find};
use crate::{
    ids::{RecurringTransactionId, UserId},
    ledger::{date_in_month, first_of_month, previous_month},
    repositories::recurring::RecurringRepository,
};

/// UC05: ends a recurring Transaction. Its future Occurrences (Projections)
/// disappear and the Occurrences that already fell stay as history.
///
/// By default the last Occurrence is the one of this month if its day has
/// already come, otherwise the one of the previous month. The user may pick a
/// later `last_month`; one that would remove an Occurrence that already fell
/// is refused.
pub struct EndRecurringTransactionUseCase {
    recurring: RecurringRepository,
}

impl EndRecurringTransactionUseCase {
    pub fn new(recurring: RecurringRepository) -> Self {
        Self { recurring }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
        today: NaiveDate,
        last_month: Option<NaiveDate>,
    ) -> Result<RecurringView, RecurringError> {
        let current = find(&self.recurring, user_id, id).await?;
        if current.definition.end_month.is_some() {
            return Err(RecurringError::Ended);
        }
        let this_month = first_of_month(today);
        let this_month_fell = date_in_month(this_month, current.definition.day_of_month) <= today;
        let latest_history = if this_month_fell {
            this_month
        } else {
            previous_month(this_month).unwrap_or(this_month)
        };
        let last_month = match last_month.map(first_of_month) {
            Some(chosen) if chosen < latest_history => {
                return Err(RecurringError::Invalid {
                    field: "last_month",
                    rule: "last_month_before_history",
                });
            }
            Some(chosen) => chosen,
            None => latest_history,
        };
        self.recurring
            .end(user_id, id, last_month)
            .await
            .map_err(RecurringError::internal)?;
        let ended = find(&self.recurring, user_id, id).await?;
        Ok(RecurringView::new(ended, today))
    }
}
