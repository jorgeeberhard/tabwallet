//! The calendar month, the one type every month calculation of the ledger
//! (invoices, recurring Transactions, reports, the projection horizon) and the
//! `YYYY-MM` text of the API go through.

use chrono::{Datelike, NaiveDate};

/// A calendar month, such as 2026-10.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month {
    year: i32,
    month: u32,
}

impl Month {
    pub fn new(year: i32, month: u32) -> Option<Self> {
        NaiveDate::from_ymd_opt(year, month, 1).map(|_| Self { year, month })
    }

    /// The month a date belongs to.
    pub fn of(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    /// Parses `YYYY-MM`.
    pub fn parse(text: &str) -> Option<Self> {
        let (year, month) = text.split_once('-')?;
        if year.len() != 4 || month.len() != 2 {
            return None;
        }
        if !year
            .chars()
            .chain(month.chars())
            .all(|c| c.is_ascii_digit())
        {
            return None;
        }
        Self::new(year.parse().ok()?, month.parse().ok()?)
    }

    pub fn first_day(self) -> NaiveDate {
        self.checked_first_day().expect("a valid month")
    }

    /// The first day, or `None` when the month is outside the dates `chrono`
    /// supports.
    pub fn checked_first_day(self) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
    }

    /// The month `months` after this one (before it, if negative).
    pub fn plus(self, months: i32) -> Self {
        let index = self.year * 12 + self.month as i32 - 1 + months;
        Self {
            year: index.div_euclid(12),
            month: index.rem_euclid(12) as u32 + 1,
        }
    }

    pub fn next(self) -> Self {
        self.plus(1)
    }

    pub fn prev(self) -> Self {
        self.plus(-1)
    }

    /// The last day of this month.
    pub fn last_day(self) -> NaiveDate {
        self.next()
            .first_day()
            .pred_opt()
            .expect("a month has a last day")
    }

    /// The date of `day` in this month; in a shorter month, its last day.
    pub fn day_or_last(self, day: u32) -> NaiveDate {
        let last = self.last_day();
        self.first_day()
            .with_day(day.min(last.day()))
            .expect("a day within the month")
    }
}

impl std::fmt::Display for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month(year: i32, month: u32) -> Month {
        Month::new(year, month).unwrap()
    }

    #[test]
    fn moves_across_year_boundaries_in_both_directions() {
        assert_eq!(month(2026, 12).next(), month(2027, 1));
        assert_eq!(month(2026, 1).prev(), month(2025, 12));
        assert_eq!(month(2026, 11).plus(14), month(2028, 1));
        assert_eq!(month(2026, 3).plus(-14), month(2025, 1));
        assert_eq!(month(2026, 5).plus(0), month(2026, 5));
    }

    #[test]
    fn knows_its_last_day() {
        assert_eq!(month(2026, 2).last_day().to_string(), "2026-02-28");
        assert_eq!(month(2028, 2).last_day().to_string(), "2028-02-29");
        assert_eq!(month(2026, 12).last_day().to_string(), "2026-12-31");
    }

    #[test]
    fn reads_and_writes_year_and_month_text() {
        assert_eq!(Month::parse("2026-03"), Some(month(2026, 3)));
        assert_eq!(month(2026, 3).to_string(), "2026-03");
        for malformed in [
            "2026-3", "26-03", "2026-13", "2026-00", "2026/03", "abcd-ef", "+026-03",
        ] {
            assert_eq!(Month::parse(malformed), None, "{malformed}");
        }
    }
}
