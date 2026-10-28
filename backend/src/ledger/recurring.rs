//! Recurring Transactions inside the ledger: how a stored definition turns
//! into Occurrences. Pure, like the rest of the ledger: nothing here is read
//! from or written to the database, and Occurrences are derived again on every
//! call (ADR-0002).

use chrono::{Datelike, NaiveDate};

use super::FlowKind;
use crate::ids::RecurringTransactionId;

/// The first day of the month `date` belongs to. Months are always handled as
/// the date of their first day.
pub fn first_of_month(date: NaiveDate) -> NaiveDate {
    date.with_day(1).expect("day 1 exists in every month")
}

/// The first day of the month after the one `month` belongs to.
pub fn next_month(month: NaiveDate) -> Option<NaiveDate> {
    let month = first_of_month(month);
    if month.month() == 12 {
        NaiveDate::from_ymd_opt(month.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(month.year(), month.month() + 1, 1)
    }
}

/// The first day of the month before the one `month` belongs to.
pub fn previous_month(month: NaiveDate) -> Option<NaiveDate> {
    let month = first_of_month(month);
    if month.month() == 1 {
        NaiveDate::from_ymd_opt(month.year() - 1, 12, 1)
    } else {
        NaiveDate::from_ymd_opt(month.year(), month.month() - 1, 1)
    }
}

/// The last day of the month `month` belongs to.
pub fn last_day_of_month(month: NaiveDate) -> NaiveDate {
    match next_month(month) {
        Some(next) => next.pred_opt().expect("a month has a last day"),
        None => NaiveDate::MAX,
    }
}

/// The day an Occurrence falls on in `month`: `day_of_month`, or the last day
/// of the month when it is shorter than that.
pub fn date_in_month(month: NaiveDate, day_of_month: u32) -> NaiveDate {
    let last_day = last_day_of_month(month);
    first_of_month(month)
        .with_day(day_of_month)
        .filter(|date| *date <= last_day)
        .unwrap_or(last_day)
}

/// An amount in force from a month on, until the next version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmountFrom {
    /// First day of the first month the amount applies to.
    pub month: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// What the user did to one single Occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccurrenceChange {
    /// The user deleted this Occurrence only.
    Skipped,
    /// The user edited the amount of this Occurrence only.
    Amount(i64),
}

/// The only thing stored about an Occurrence: that the user changed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccurrenceException {
    /// First day of the month of the Occurrence.
    pub month: NaiveDate,
    pub change: OccurrenceChange,
}

/// A recurring Transaction (Gasto fixo or Receita fixa) as the ledger needs
/// it: when it happens, the amount in each month and the user's exceptions.
#[derive(Clone, Debug)]
pub struct RecurringDefinition {
    pub id: RecurringTransactionId,
    pub kind: FlowKind,
    /// 1 to 31. In shorter months the Occurrence falls on the last day.
    pub day_of_month: u32,
    /// First day of the first month with an Occurrence.
    pub start_month: NaiveDate,
    /// First day of the last month with an Occurrence, once ended.
    pub end_month: Option<NaiveDate>,
    /// Amount versions; changing the amount adds one from the month of the
    /// change, so earlier months keep theirs.
    pub amounts: Vec<AmountFrom>,
    pub exceptions: Vec<OccurrenceException>,
}

/// The Transaction one recurring Transaction makes in one month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Occurrence {
    pub recurring_id: RecurringTransactionId,
    pub kind: FlowKind,
    /// First day of its month: with the recurring id, it identifies the
    /// Occurrence.
    pub month: NaiveDate,
    /// The day it falls on: the defined day, or the last day of a shorter
    /// month.
    pub date: NaiveDate,
    pub amount_cents: i64,
    /// The user edited the amount of this Occurrence only.
    pub edited: bool,
}

impl RecurringDefinition {
    /// The amount the definition has in `month`: the latest version that
    /// started on or before it.
    pub fn amount_in_force(&self, month: NaiveDate) -> Option<i64> {
        let month = first_of_month(month);
        self.amounts
            .iter()
            .filter(|version| version.month <= month)
            .max_by_key(|version| version.month)
            .map(|version| version.amount_cents)
    }

    /// The amount to show for the definition on `today`: the one in force this
    /// month, or in its first (or last) month if it has not started (or has
    /// ended).
    pub fn reference_amount(&self, today: NaiveDate) -> Option<i64> {
        let mut month = first_of_month(today).max(self.start_month);
        if let Some(end) = self.end_month {
            month = month.min(end.max(self.start_month));
        }
        self.amount_in_force(month)
    }

    /// The Occurrence of one month, or `None` if there is none: before the
    /// start, after the end, or deleted by the user.
    pub fn occurrence_in(&self, month: NaiveDate) -> Option<Occurrence> {
        let month = first_of_month(month);
        if month < self.start_month || self.end_month.is_some_and(|end| month > end) {
            return None;
        }
        let exception = self
            .exceptions
            .iter()
            .find(|exception| exception.month == month)
            .map(|exception| exception.change);
        let (amount_cents, edited) = match exception {
            Some(OccurrenceChange::Skipped) => return None,
            Some(OccurrenceChange::Amount(amount)) => (amount, true),
            None => (self.amount_in_force(month)?, false),
        };
        Some(Occurrence {
            recurring_id: self.id,
            kind: self.kind,
            month,
            date: date_in_month(month, self.day_of_month),
            amount_cents,
            edited,
        })
    }

    /// The Occurrences that fall on a day from `from` to `to`, both inclusive,
    /// oldest first.
    pub fn occurrences_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<Occurrence> {
        let mut month = first_of_month(from).max(self.start_month);
        let mut last = first_of_month(to);
        if let Some(end) = self.end_month {
            last = last.min(end);
        }
        let mut occurrences = Vec::new();
        while month <= last {
            if let Some(occurrence) = self.occurrence_in(month)
                && occurrence.date >= from
                && occurrence.date <= to
            {
                occurrences.push(occurrence);
            }
            match next_month(month) {
                Some(next) => month = next,
                None => break,
            }
        }
        occurrences
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn definition(day_of_month: u32, start: NaiveDate) -> RecurringDefinition {
        RecurringDefinition {
            id: RecurringTransactionId(Uuid::nil()),
            kind: FlowKind::Expense,
            day_of_month,
            start_month: first_of_month(start),
            end_month: None,
            amounts: vec![AmountFrom {
                month: first_of_month(start),
                amount_cents: 1_000,
            }],
            exceptions: Vec::new(),
        }
    }

    #[test]
    fn falls_on_the_last_day_of_shorter_months() {
        let rent = definition(31, date(2024, 1, 1));

        let dates: Vec<NaiveDate> = rent
            .occurrences_between(date(2024, 1, 1), date(2024, 4, 30))
            .iter()
            .map(|occurrence| occurrence.date)
            .collect();

        // 2024 is a leap year.
        assert_eq!(
            dates,
            vec![
                date(2024, 1, 31),
                date(2024, 2, 29),
                date(2024, 3, 31),
                date(2024, 4, 30)
            ]
        );
    }

    #[test]
    fn a_new_amount_applies_from_its_month_on_and_leaves_earlier_months_alone() {
        let mut rent = definition(5, date(2026, 1, 1));
        rent.amounts.push(AmountFrom {
            month: date(2026, 3, 1),
            amount_cents: 1_200,
        });

        let amounts: Vec<i64> = rent
            .occurrences_between(date(2026, 1, 1), date(2026, 4, 30))
            .iter()
            .map(|occurrence| occurrence.amount_cents)
            .collect();

        assert_eq!(amounts, vec![1_000, 1_000, 1_200, 1_200]);
    }

    #[test]
    fn a_skipped_month_has_no_occurrence_and_an_edited_one_keeps_only_its_own_amount() {
        let mut rent = definition(5, date(2026, 1, 1));
        rent.exceptions = vec![
            OccurrenceException {
                month: date(2026, 2, 1),
                change: OccurrenceChange::Skipped,
            },
            OccurrenceException {
                month: date(2026, 3, 1),
                change: OccurrenceChange::Amount(777),
            },
        ];

        let found: Vec<(u32, i64, bool)> = rent
            .occurrences_between(date(2026, 1, 1), date(2026, 4, 30))
            .iter()
            .map(|o| (o.month.month(), o.amount_cents, o.edited))
            .collect();

        assert_eq!(
            found,
            vec![(1, 1_000, false), (3, 777, true), (4, 1_000, false)]
        );
    }

    #[test]
    fn stops_after_the_end_month_and_only_lists_days_inside_the_window() {
        let mut rent = definition(20, date(2026, 1, 1));
        rent.end_month = Some(date(2026, 3, 1));

        let months: Vec<u32> = rent
            .occurrences_between(date(2026, 2, 1), date(2026, 12, 31))
            .iter()
            .map(|occurrence| occurrence.month.month())
            .collect();
        assert_eq!(months, vec![2, 3]);

        // The window starts after the 20th of February.
        let months: Vec<u32> = rent
            .occurrences_between(date(2026, 2, 21), date(2026, 12, 31))
            .iter()
            .map(|occurrence| occurrence.month.month())
            .collect();
        assert_eq!(months, vec![3]);
    }
}
