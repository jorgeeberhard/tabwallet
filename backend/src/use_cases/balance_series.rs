//! The balance over time: the history of the Current Balance up to
//! today and the Projected Balance up to the 12-month horizon. Each use case
//! feeds the ledger with what is stored and asks it for a series.

use chrono::{Days, Months, NaiveDate};

use crate::{
    ids::UserId,
    ledger::{BalancePoint, Granularity, Ledger, SeriesError, first_of_month, projection_horizon},
    repositories::{
        card_purchases::CardPurchaseRepository, credit_cards::CreditCardRepository,
        opening_balance::OpeningBalanceRepository, recurring::RecurringRepository,
        transactions::TransactionRepository,
    },
};

use super::ledger_loader::LedgerLoader;

/// How many days a daily history shows when the caller gives no start.
pub const DEFAULT_HISTORY_DAYS: u64 = 30;
/// How many months a monthly history shows when the caller gives no start.
pub const DEFAULT_HISTORY_MONTHS: u32 = 12;

/// Why a balance series use case failed. Unaware of HTTP: the handler layer
/// decides the status code.
#[derive(Debug, thiserror::Error)]
pub enum BalanceSeriesError {
    /// A date of the period breaks a rule. `field` is the request field and
    /// `rule` a stable code for it.
    #[error("invalid period: {field} {rule}")]
    InvalidPeriod {
        field: &'static str,
        rule: &'static str,
    },
    #[error("unexpected failure: {0}")]
    Internal(String),
}

internal_error!(BalanceSeriesError);

impl From<SeriesError> for BalanceSeriesError {
    fn from(error: SeriesError) -> Self {
        let (field, rule) = match error {
            SeriesError::PeriodOrder => ("to", "period_order"),
            SeriesError::PeriodTooLong => ("from", "period_too_long"),
            SeriesError::HistoryInFuture => ("to", "history_in_future"),
            SeriesError::ProjectionInPast => ("to", "projection_in_past"),
            SeriesError::BeyondHorizon => ("to", "beyond_horizon"),
        };
        Self::InvalidPeriod { field, rule }
    }
}

/// The ledger over everything stored about the user.
async fn ledger_of(loader: &LedgerLoader, user_id: UserId) -> Result<Ledger, BalanceSeriesError> {
    loader
        .facts(user_id)
        .await
        .map(Ledger::new)
        .map_err(BalanceSeriesError::internal)
}

/// What the user asked of the history. Everything but the granularity is
/// optional.
#[derive(Clone, Copy, Debug)]
pub struct HistoryQuery {
    pub granularity: Granularity,
    /// First day, inclusive. Default: the last 30 days (or 12 months) up to
    /// `to`.
    pub from: Option<NaiveDate>,
    /// Last day, inclusive, up to today. Default: today.
    pub to: Option<NaiveDate>,
}

/// The history, with the period it actually covers.
#[derive(Clone, Debug)]
pub struct BalanceHistory {
    pub granularity: Granularity,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub points: Vec<BalancePoint>,
}

/// The Current Balance at the end of each day or month of a
/// period that ends today at the latest.
pub struct GetBalanceHistoryUseCase {
    loader: LedgerLoader,
}

impl GetBalanceHistoryUseCase {
    pub fn new(
        transactions: TransactionRepository,
        opening_balance: OpeningBalanceRepository,
        recurring: RecurringRepository,
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
    ) -> Self {
        Self {
            loader: LedgerLoader::new(transactions, opening_balance, recurring, cards, purchases),
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
        query: HistoryQuery,
    ) -> Result<BalanceHistory, BalanceSeriesError> {
        let to = query.to.unwrap_or(today);
        let from = query
            .from
            .unwrap_or_else(|| default_history_start(to, query.granularity));
        let points = ledger_of(&self.loader, user_id).await?.balance_history(
            from,
            to,
            query.granularity,
            today,
        )?;
        Ok(BalanceHistory {
            granularity: query.granularity,
            from,
            to,
            points,
        })
    }
}

/// Where a history starts when the caller does not say: a month of days or a
/// year of months back from `to`.
fn default_history_start(to: NaiveDate, granularity: Granularity) -> NaiveDate {
    match granularity {
        Granularity::Day => to
            .checked_sub_days(Days::new(DEFAULT_HISTORY_DAYS - 1))
            .unwrap_or(NaiveDate::MIN),
        Granularity::Month => to
            .checked_sub_months(Months::new(DEFAULT_HISTORY_MONTHS - 1))
            .map(first_of_month)
            .unwrap_or(NaiveDate::MIN),
    }
}

/// The projection, with the Current Balance it starts from.
#[derive(Clone, Debug)]
pub struct ProjectedBalance {
    pub granularity: Granularity,
    pub today: NaiveDate,
    pub to: NaiveDate,
    pub current_balance_cents: i64,
    pub points: Vec<BalancePoint>,
}

/// The Projected Balance from today up to a date at most 12
/// months ahead. Defaults to the horizon itself.
pub struct GetProjectedBalanceUseCase {
    loader: LedgerLoader,
}

impl GetProjectedBalanceUseCase {
    pub fn new(
        transactions: TransactionRepository,
        opening_balance: OpeningBalanceRepository,
        recurring: RecurringRepository,
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
    ) -> Self {
        Self {
            loader: LedgerLoader::new(transactions, opening_balance, recurring, cards, purchases),
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
        granularity: Granularity,
        to: Option<NaiveDate>,
    ) -> Result<ProjectedBalance, BalanceSeriesError> {
        let to = to.unwrap_or_else(|| projection_horizon(today));
        let ledger = ledger_of(&self.loader, user_id).await?;
        let points = ledger.projected_balance(to, granularity, today)?;
        Ok(ProjectedBalance {
            granularity,
            today,
            to,
            current_balance_cents: ledger.current_balance(today),
            points,
        })
    }
}
