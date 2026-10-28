use chrono::NaiveDate;

use crate::{
    ids::UserId,
    ledger::{Ledger, LedgerFacts},
    repositories::{
        opening_balance::OpeningBalanceRepository, transactions::TransactionRepository,
    },
};

/// Why a balance use case failed. Unaware of HTTP: the handler layer decides
/// the status code.
#[derive(Debug, thiserror::Error)]
pub enum BalanceError {
    /// The Opening Balance is informed only once.
    #[error("the opening balance was already informed")]
    OpeningBalanceAlreadySet,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl BalanceError {
    fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

/// The user's balance on one day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BalanceReport {
    pub current_balance_cents: i64,
    /// `None` if the user never informed one.
    pub opening_balance_cents: Option<i64>,
}

/// UC10 (partial): the Current Balance on `today`, with the Opening Balance
/// it includes. Feeds the ledger with what is stored and asks it.
pub struct GetBalanceUseCase {
    transactions: TransactionRepository,
    opening_balance: OpeningBalanceRepository,
}

impl GetBalanceUseCase {
    pub fn new(
        transactions: TransactionRepository,
        opening_balance: OpeningBalanceRepository,
    ) -> Self {
        Self {
            transactions,
            opening_balance,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
    ) -> Result<BalanceReport, BalanceError> {
        let opening_balance_cents = self
            .opening_balance
            .find(user_id)
            .await
            .map_err(BalanceError::internal)?;
        let cash_flows = self
            .transactions
            .cash_flows(user_id)
            .await
            .map_err(BalanceError::internal)?;
        let ledger = Ledger::new(LedgerFacts {
            opening_balance_cents,
            cash_flows,
        });
        Ok(BalanceReport {
            current_balance_cents: ledger.current_balance(today),
            opening_balance_cents,
        })
    }
}

/// UC22: records the Opening Balance, once. It may be zero or negative.
pub struct SetOpeningBalanceUseCase {
    opening_balance: OpeningBalanceRepository,
}

impl SetOpeningBalanceUseCase {
    pub fn new(opening_balance: OpeningBalanceRepository) -> Self {
        Self { opening_balance }
    }

    pub async fn execute(&self, user_id: UserId, amount_cents: i64) -> Result<(), BalanceError> {
        match self.opening_balance.set_once(user_id, amount_cents).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(BalanceError::OpeningBalanceAlreadySet),
            Err(error) => Err(BalanceError::internal(error)),
        }
    }
}
