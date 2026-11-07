//! Balance over time: the history of the Current Balance up to today and the
//! Projected Balance up to the horizon (UC10).
//!
//! Both are series of points, one per day or per month, and both come from the
//! same running total of dated cash flows. They differ in what they promise:
//! a history point is the Current Balance the user had at the end of that day
//! or month, a projected point is the balance expected at the end of it.

use chrono::NaiveDate;

use super::{Ledger, first_of_month, last_day_of_month, next_month, projection_horizon, signed};

/// The most points a series may have. It keeps one request from asking for
/// centuries of days; three years of days or eighty years of months fit.
pub const MAX_SERIES_POINTS: usize = 1000;

/// How far apart the points of a series are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Granularity {
    Day,
    Month,
}

/// The balance at the end of one day. For a monthly series `date` is the last
/// day of the month, or the last day of the period when that comes first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BalancePoint {
    pub date: NaiveDate,
    /// Cents; may be negative.
    pub balance_cents: i64,
}

/// Why a series was refused. Each variant names the rule broken; the caller
/// decides which request field it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeriesError {
    /// The period starts after it ends.
    PeriodOrder,
    /// The period has more than [`MAX_SERIES_POINTS`] points.
    PeriodTooLong,
    /// A history reaches only up to today.
    HistoryInFuture,
    /// A projection starts today: it cannot end before it.
    ProjectionInPast,
    /// A projection reaches only up to [`projection_horizon`].
    BeyondHorizon,
}

impl Ledger {
    /// The history of the Current Balance: one point per day (or month) from
    /// `from` to `to`, each the balance at the end of that day (or month), with
    /// the Opening Balance and the Occurrences that had fallen by then. `to`
    /// cannot be after `today`; Projections are not history. A month point is
    /// the end of that month, cut at `to`, so the month that holds `to` shows
    /// the balance at `to`.
    pub fn balance_history(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        granularity: Granularity,
        today: NaiveDate,
    ) -> Result<Vec<BalancePoint>, SeriesError> {
        if to > today {
            return Err(SeriesError::HistoryInFuture);
        }
        let dates = point_dates(from, to, granularity)?;
        Ok(self.balances_at(&dates))
    }

    /// The Projected Balance from today up to `to`: one point per day (or
    /// month), each the Current Balance plus the incomes and expenses outside
    /// the credit card dated after `today` up to the point, Projections and
    /// recurring Occurrences included. The first point is today (the Current
    /// Balance, for days) or the end of this month (for months). `to` cannot
    /// pass [`projection_horizon`]. Purchases not yet recorded are not known
    /// and stay out.
    ///
    /// Invoices are not part of it yet: the ticket on invoice payments
    /// subtracts, at each point, the remaining amount of the invoices due up
    /// to that date, inside [`Ledger::projected_balances_at`].
    pub fn projected_balance(
        &self,
        to: NaiveDate,
        granularity: Granularity,
        today: NaiveDate,
    ) -> Result<Vec<BalancePoint>, SeriesError> {
        if to < today {
            return Err(SeriesError::ProjectionInPast);
        }
        if to > projection_horizon(today) {
            return Err(SeriesError::BeyondHorizon);
        }
        let dates = point_dates(today, to, granularity)?;
        Ok(self.projected_balances_at(&dates))
    }

    /// The projected balance at the end of each of `dates` (ascending, none
    /// before today). Without invoices it equals the running total of cash
    /// flows; with them, the amount still owed on invoices due by each date
    /// comes off here.
    fn projected_balances_at(&self, dates: &[NaiveDate]) -> Vec<BalancePoint> {
        self.balances_at(dates)
    }

    /// The balance at the end of each of `dates`, which must be ascending:
    /// the Opening Balance plus every flow and Occurrence dated up to it.
    fn balances_at(&self, dates: &[NaiveDate]) -> Vec<BalancePoint> {
        let Some(&last) = dates.last() else {
            return Vec::new();
        };
        let mut events: Vec<(NaiveDate, i64)> = self
            .facts
            .cash_flows
            .iter()
            .filter(|flow| flow.date <= last)
            .map(|flow| (flow.date, signed(flow.kind, flow.amount_cents)))
            .chain(
                self.occurrences(NaiveDate::MIN, last)
                    .iter()
                    .map(|occurrence| {
                        (
                            occurrence.date,
                            signed(occurrence.kind, occurrence.amount_cents),
                        )
                    }),
            )
            .collect();
        events.sort_by_key(|(date, _)| *date);

        let mut balance = self.facts.opening_balance_cents.unwrap_or(0);
        let mut pending = events.into_iter().peekable();
        dates
            .iter()
            .map(|&date| {
                while let Some((_, cents)) = pending.next_if(|(day, _)| *day <= date) {
                    balance += cents;
                }
                BalancePoint {
                    date,
                    balance_cents: balance,
                }
            })
            .collect()
    }
}

/// The days a series has a point on, ascending, from `from` to `to`.
fn point_dates(
    from: NaiveDate,
    to: NaiveDate,
    granularity: Granularity,
) -> Result<Vec<NaiveDate>, SeriesError> {
    if from > to {
        return Err(SeriesError::PeriodOrder);
    }
    match granularity {
        Granularity::Day => {
            let days = (to - from).num_days();
            if days >= MAX_SERIES_POINTS as i64 {
                return Err(SeriesError::PeriodTooLong);
            }
            Ok(from.iter_days().take_while(|day| *day <= to).collect())
        }
        Granularity::Month => {
            let mut dates = Vec::new();
            let mut month = Some(first_of_month(from));
            while let Some(first) = month.filter(|first| *first <= to) {
                if dates.len() == MAX_SERIES_POINTS {
                    return Err(SeriesError::PeriodTooLong);
                }
                dates.push(last_day_of_month(first).min(to));
                month = next_month(first);
            }
            Ok(dates)
        }
    }
}
