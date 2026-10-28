use chrono::NaiveDate;

use super::{RecurringError, find};
use crate::{
    ids::{RecurringTransactionId, UserId},
    ledger::{Ledger, LedgerFacts, Occurrence, OccurrenceChange, first_of_month},
    repositories::recurring::{RecurringRepository, RecurringTransaction},
};

/// One Occurrence with the recurring Transaction it belongs to.
#[derive(Clone, Debug)]
pub struct OccurrenceView {
    pub recurring: RecurringTransaction,
    pub occurrence: Occurrence,
}

/// The Occurrence of `month`, asked of the ledger, or `OccurrenceNotFound`.
fn occurrence_of(
    recurring: &RecurringTransaction,
    month: NaiveDate,
) -> Result<Occurrence, RecurringError> {
    Ledger::new(LedgerFacts {
        recurring: vec![recurring.definition.clone()],
        ..LedgerFacts::default()
    })
    .occurrence(recurring.id, month)
    .ok_or(RecurringError::OccurrenceNotFound)
}

/// UC07: deletes one single Occurrence. Only an exception is stored; the
/// other months of the recurring Transaction are not affected.
pub struct SkipOccurrenceUseCase {
    recurring: RecurringRepository,
}

impl SkipOccurrenceUseCase {
    pub fn new(recurring: RecurringRepository) -> Self {
        Self { recurring }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
        month: NaiveDate,
    ) -> Result<(), RecurringError> {
        let month = first_of_month(month);
        let current = find(&self.recurring, user_id, id).await?;
        occurrence_of(&current, month)?;
        self.recurring
            .set_exception(user_id, id, month, OccurrenceChange::Skipped)
            .await
            .map_err(RecurringError::internal)
    }
}

/// UC06: edits the amount of one single Occurrence. Only an exception is
/// stored; the other months keep the amount of the recurring Transaction.
pub struct EditOccurrenceUseCase {
    recurring: RecurringRepository,
}

impl EditOccurrenceUseCase {
    pub fn new(recurring: RecurringRepository) -> Self {
        Self { recurring }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: RecurringTransactionId,
        month: NaiveDate,
        amount_cents: i64,
    ) -> Result<OccurrenceView, RecurringError> {
        let month = first_of_month(month);
        let current = find(&self.recurring, user_id, id).await?;
        occurrence_of(&current, month)?;
        self.recurring
            .set_exception(user_id, id, month, OccurrenceChange::Amount(amount_cents))
            .await
            .map_err(RecurringError::internal)?;
        let updated = find(&self.recurring, user_id, id).await?;
        let occurrence = occurrence_of(&updated, month)?;
        Ok(OccurrenceView {
            recurring: updated,
            occurrence,
        })
    }
}
