//! Credit-card invoices, derived from dates (ADR-0002, ADR-0003).
//!
//! An Invoice is never stored. It is identified by its card and its closing
//! month (the month in which it closes) and everything about it follows from
//! the card's closing and due days, the purchases and "today":
//!
//! - A purchase on or before the Closing Day goes to the invoice that closes
//!   that month, a later one to the invoice of the next month.
//! - The due date is the first date after the closing date that falls on the
//!   Due Day (in a shorter month the last day applies).
//! - State: Open until the closing date (inclusive), Closed after it until the
//!   due date, Paid from the due date on.
//!
//! A card keeps a history of its days ([`CardSchedule`]) so that changing them
//! only affects invoices still open: closed and paid invoices keep the days
//! they were closed with.

use chrono::{Datelike, NaiveDate};

use crate::ids::CreditCardId;

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
        Self::new(year.parse().ok()?, month.parse().ok()?)
    }

    pub fn first_day(self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, 1).expect("a valid month")
    }

    pub fn next(self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }

    /// The date of `day` in this month; in a shorter month, its last day.
    pub fn day_or_last(self, day: u32) -> NaiveDate {
        let last = self.next().first_day().pred_opt().expect("a valid date");
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

/// The closing and due days a card has from one closing month on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduleVersion {
    /// The first invoice (by closing month) that uses these days.
    pub effective_from: Month,
    /// 1 to 31.
    pub closing_day: u32,
    /// 1 to 31.
    pub due_day: u32,
}

/// The history of a card's closing and due days, oldest first.
#[derive(Clone, Debug)]
pub struct CardSchedule {
    versions: Vec<ScheduleVersion>,
}

impl CardSchedule {
    /// `None` if there is no version at all: a card always has at least one.
    pub fn new(mut versions: Vec<ScheduleVersion>) -> Option<Self> {
        if versions.is_empty() {
            return None;
        }
        versions.sort_by_key(|version| version.effective_from);
        Some(Self { versions })
    }

    /// The days the invoice closing in `month` uses.
    fn version_for(&self, month: Month) -> &ScheduleVersion {
        self.versions
            .iter()
            .rev()
            .find(|version| version.effective_from <= month)
            .unwrap_or(&self.versions[0])
    }

    /// The days the card has from now on: the latest version.
    pub fn latest(&self) -> &ScheduleVersion {
        self.versions.last().expect("a schedule is never empty")
    }

    pub fn closing_date(&self, month: Month) -> NaiveDate {
        month.day_or_last(self.version_for(month).closing_day)
    }

    /// The first date after the closing date that is the Due Day: in the same
    /// month when the Due Day comes after the closing date, else in the next.
    pub fn due_date(&self, month: Month) -> NaiveDate {
        let closing = self.closing_date(month);
        let due_day = self.version_for(month).due_day;
        let same_month = month.day_or_last(due_day);
        if same_month > closing {
            same_month
        } else {
            month.next().day_or_last(due_day)
        }
    }

    /// The closing month of the invoice a purchase made on `date` enters.
    pub fn invoice_month_for(&self, date: NaiveDate) -> Month {
        let month = Month::of(date);
        if date <= self.closing_date(month) {
            month
        } else {
            month.next()
        }
    }

    /// The closing month of the first invoice that is still open on `today`.
    pub fn first_open_month(&self, today: NaiveDate) -> Month {
        self.invoice_month_for(today)
    }

    /// The first closing month that a change of the days made `today` would
    /// apply to: the first open invoice, unless the new Closing Day would
    /// already have passed for it, in which case that invoice is left as it
    /// is (no purchase moves) and the change starts with the next one.
    pub fn change_takes_effect_from(&self, new_closing_day: u32, today: NaiveDate) -> Month {
        let month = self.first_open_month(today);
        if month.day_or_last(new_closing_day) < today {
            month.next()
        } else {
            month
        }
    }
}

/// How far an invoice is on `today`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvoiceState {
    /// Receives purchases: up to and including the closing date.
    Open,
    /// After the closing date, before the due date.
    Closed,
    /// From the due date on.
    Paid,
}

/// The dates and state of one invoice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvoiceCycle {
    pub closing_month: Month,
    pub closing_date: NaiveDate,
    pub due_date: NaiveDate,
    pub state: InvoiceState,
}

impl InvoiceCycle {
    /// The month everything in the invoice counts in for reports and budgets:
    /// the month of its due date.
    pub fn reference_month(&self) -> Month {
        Month::of(self.due_date)
    }
}

/// An invoice with its amounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invoice {
    pub card_id: CreditCardId,
    pub cycle: InvoiceCycle,
    /// The sum of the purchases in it.
    pub total_cents: i64,
    /// What is still to be paid. Until invoice payments exist (a later
    /// ticket), the due date is what settles an invoice: from it on the
    /// invoice is Paid and nothing remains.
    pub remaining_cents: i64,
}

/// Card purchases of one day, added up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardSpend {
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// What is stored about one card that the ledger needs.
#[derive(Clone, Debug)]
pub struct CardFacts {
    pub id: CreditCardId,
    pub schedule: CardSchedule,
    pub spending: Vec<CardSpend>,
}

impl CardFacts {
    fn cycle(&self, month: Month, today: NaiveDate) -> InvoiceCycle {
        let closing_date = self.schedule.closing_date(month);
        let due_date = self.schedule.due_date(month);
        let state = if today <= closing_date {
            InvoiceState::Open
        } else if today < due_date {
            InvoiceState::Closed
        } else {
            InvoiceState::Paid
        };
        InvoiceCycle {
            closing_month: month,
            closing_date,
            due_date,
            state,
        }
    }

    /// The dates and state of the invoice a purchase made on `date` enters.
    pub fn cycle_for_purchase(&self, date: NaiveDate, today: NaiveDate) -> InvoiceCycle {
        self.cycle(self.schedule.invoice_month_for(date), today)
    }

    fn invoice(&self, month: Month, today: NaiveDate) -> Invoice {
        let cycle = self.cycle(month, today);
        let total_cents = self
            .spending
            .iter()
            .filter(|spend| self.schedule.invoice_month_for(spend.date) == month)
            .map(|spend| spend.amount_cents)
            .sum();
        let remaining_cents = match cycle.state {
            InvoiceState::Paid => 0,
            _ => total_cents,
        };
        Invoice {
            card_id: self.id,
            cycle,
            total_cents,
            remaining_cents,
        }
    }

    /// The card's invoices, oldest first: every one that has purchases and the
    /// first open one, even when still empty.
    pub fn invoices(&self, today: NaiveDate) -> Vec<Invoice> {
        let mut months: Vec<Month> = self
            .spending
            .iter()
            .map(|spend| self.schedule.invoice_month_for(spend.date))
            .collect();
        months.push(self.schedule.first_open_month(today));
        months.sort();
        months.dedup();
        months
            .into_iter()
            .map(|month| self.invoice(month, today))
            .collect()
    }

    pub fn find_invoice(&self, month: Month, today: NaiveDate) -> Option<Invoice> {
        self.invoices(today)
            .into_iter()
            .find(|invoice| invoice.cycle.closing_month == month)
    }
}
