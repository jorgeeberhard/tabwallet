//! Source of "today".
//!
//! A transaction date is only a day, and "today" is always the calendar day
//! in America/Sao_Paulo (see GLOSSARY.md, "Saldo atual"). Business code asks a
//! `Clock` instead of reading the system time, so tests can move "today".

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::America::Sao_Paulo;

pub trait Clock: Send + Sync {
    /// The current calendar day in America/Sao_Paulo.
    fn today(&self) -> NaiveDate;
}

/// The production clock, backed by the system time.
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> NaiveDate {
        sao_paulo_date(Utc::now())
    }
}

/// The calendar day in America/Sao_Paulo at the given instant.
pub fn sao_paulo_date(instant: DateTime<Utc>) -> NaiveDate {
    instant.with_timezone(&Sao_Paulo).date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(text: &str) -> DateTime<Utc> {
        text.parse().unwrap()
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn day_changes_at_midnight_in_sao_paulo_not_in_utc() {
        // 02:59 UTC is 23:59 of the previous day in Sao Paulo (UTC-3).
        assert_eq!(
            sao_paulo_date(instant("2026-10-27T02:59:59Z")),
            date(2026, 10, 26)
        );
        assert_eq!(
            sao_paulo_date(instant("2026-10-27T03:00:00Z")),
            date(2026, 10, 27)
        );
    }

    #[test]
    fn follows_the_time_zone_database_for_past_daylight_saving_time() {
        // Brazil still observed DST in 2018 (UTC-2): 02:30 UTC is already 00:30.
        assert_eq!(
            sao_paulo_date(instant("2018-11-05T02:30:00Z")),
            date(2018, 11, 5)
        );
    }
}
