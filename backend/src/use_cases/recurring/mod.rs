//! Recurring Transactions (Gastos fixos and Receitas fixas, UC05) and their
//! Occurrences (UC06 and UC07). The Occurrences are never stored: only the
//! definition and the exceptions the user makes are (ADR-0002).

mod create;
mod end;
mod list;
mod occurrence;
mod update;

use chrono::NaiveDate;

pub use create::{CreateRecurringTransactionUseCase, NewRecurring};
pub use end::EndRecurringTransactionUseCase;
pub use list::ListRecurringTransactionsUseCase;
pub use occurrence::{EditOccurrenceUseCase, OccurrenceView, SkipOccurrenceUseCase};
pub use update::{RecurringChanges, UpdateRecurringTransactionUseCase};

use super::{CategoryProblem, TransactionError};
use crate::{ids::RecurringTransactionId, repositories::recurring::RecurringTransaction};
use crate::{
    ids::UserId, ledger::projection_horizon, repositories::recurring::RecurringRepository,
};

/// Why a recurring Transaction use case failed. Unaware of HTTP: the handler
/// layer decides the status code.
#[derive(Debug, thiserror::Error)]
pub enum RecurringError {
    /// There is no such recurring Transaction, or it belongs to another user.
    #[error("recurring transaction not found")]
    NotFound,
    /// The recurring Transaction has no Occurrence in that month: it is
    /// before the start, after the end, or the user already deleted it.
    #[error("occurrence not found")]
    OccurrenceNotFound,
    /// The recurring Transaction was already ended and cannot be changed.
    #[error("the recurring transaction has ended")]
    Ended,
    #[error("the category cannot be used: {0:?}")]
    InvalidCategory(CategoryProblem),
    /// A field breaks a rule: the name of the field and the rule's code.
    #[error("invalid {field}: {rule}")]
    Invalid {
        field: &'static str,
        rule: &'static str,
    },
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl RecurringError {
    pub fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<TransactionError> for RecurringError {
    fn from(error: TransactionError) -> Self {
        match error {
            TransactionError::NotFound => Self::NotFound,
            TransactionError::InvalidCategory(problem) => Self::InvalidCategory(problem),
            TransactionError::Internal(cause) => Self::Internal(cause),
        }
    }
}

/// A recurring Transaction as shown on `today`.
#[derive(Clone, Debug)]
pub struct RecurringView {
    pub recurring: RecurringTransaction,
    /// The amount in force this month (or in its first or last month if it has
    /// not started or has already ended), in cents.
    pub amount_cents: i64,
    /// The day of the next Occurrence after today, within the projection
    /// window; `None` if it ended or has none left.
    pub next_date: Option<NaiveDate>,
}

impl RecurringView {
    fn new(recurring: RecurringTransaction, today: NaiveDate) -> Self {
        let amount_cents = recurring.definition.reference_amount(today).unwrap_or(0);
        let next_date = today.succ_opt().and_then(|tomorrow| {
            recurring
                .definition
                .occurrences_between(tomorrow, projection_horizon(today))
                .first()
                .map(|occurrence| occurrence.date)
        });
        Self {
            recurring,
            amount_cents,
            next_date,
        }
    }
}

/// The user's recurring Transaction, or `NotFound`.
async fn find(
    recurring: &RecurringRepository,
    user_id: UserId,
    id: RecurringTransactionId,
) -> Result<RecurringTransaction, RecurringError> {
    recurring
        .find(user_id, id)
        .await
        .map_err(RecurringError::internal)?
        .ok_or(RecurringError::NotFound)
}
