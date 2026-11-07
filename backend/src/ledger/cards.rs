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
//! Paying an invoice (UC17, ADR-0003): the user may make Early Payments, total
//! or partial, while the invoice is Open or Closed. On the due date the ledger
//! derives an automatic Invoice Payment of whatever is still unpaid and the
//! invoice becomes Paid. Early Payments are stored; the automatic one never
//! is. Neither is an expense: they leave the balance on their date and are
//! reported as [`InvoicePayment`]s, apart from card purchases.
//!
//! A card keeps a history of its days ([`CardSchedule`]) so that changing them
//! only affects invoices still open: closed and paid invoices keep the days
//! they were closed with.

use chrono::{Datelike, NaiveDate};

use crate::ids::{CreditCardId, InvoicePaymentId};

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

    /// The month `months` after this one.
    pub fn plus(self, months: u32) -> Self {
        let index = self.year * 12 + self.month as i32 - 1 + months as i32;
        Self {
            year: index.div_euclid(12),
            month: index.rem_euclid(12) as u32 + 1,
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
    /// What has left the balance for this invoice: the Early Payments, and
    /// once the due date has come, the automatic payment of the rest.
    pub paid_cents: i64,
    /// What is still to be paid: the total less the Early Payments, never
    /// negative. Zero once the invoice is Paid (the due date settles it); an
    /// Open or Closed invoice paid in full early also has zero remaining but
    /// keeps its state until the due date, so purchases may still raise it.
    pub remaining_cents: i64,
}

/// Where a payment of an invoice came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaymentOrigin {
    /// Made by the user before the due date and stored.
    Early(InvoicePaymentId),
    /// Derived on the due date from what was still unpaid; never stored.
    Automatic,
}

/// Money that left (or will leave) the balance to pay an invoice. It is not an
/// expense: the spending was counted when the purchases were made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvoicePayment {
    pub card_id: CreditCardId,
    /// The invoice it pays, by its closing month.
    pub closing_month: Month,
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
    pub origin: PaymentOrigin,
}

/// An Early Payment as stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaymentFact {
    pub id: InvoicePaymentId,
    /// The closing month of the invoice it pays.
    pub closing_month: Month,
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// Card purchases of one day, added up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardSpend {
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// One Installment's weight on the invoice it is in. Unlike a single
/// purchase, whose invoice follows from its date, an Installment is
/// allocated to an invoice when it is stored, so that it can be moved alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallmentCharge {
    /// The closing month of the invoice the Installment is in.
    pub invoice_month: Month,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// One Installment of an installment purchase that has not been stored yet:
/// its number, its amount and the invoice it would go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedInstallment {
    /// 1 for the first.
    pub number: u32,
    pub amount_cents: i64,
    pub cycle: InvoiceCycle,
}

/// The fewest and the most Installments an installment purchase can have.
pub const MIN_INSTALLMENTS: u32 = 2;
pub const MAX_INSTALLMENTS: u32 = 48;

/// Splits `total_cents` in `count` amounts of exact cents that add up to the
/// total, the cents left over by the division going to the first one: R$ 100
/// in 3 is 33,34 + 33,33 + 33,33. `None` if `count` is zero or the total
/// does not give every amount at least one cent.
pub fn split_installments(total_cents: i64, count: u32) -> Option<Vec<i64>> {
    let count_cents = i64::from(count);
    if count == 0 || total_cents < count_cents {
        return None;
    }
    let share = total_cents / count_cents;
    let leftover = total_cents % count_cents;
    Some(
        (0..count)
            .map(|index| if index == 0 { share + leftover } else { share })
            .collect(),
    )
}

/// What is stored about one card that the ledger needs.
#[derive(Clone, Debug)]
pub struct CardFacts {
    pub id: CreditCardId,
    pub schedule: CardSchedule,
    pub spending: Vec<CardSpend>,
    /// The card's Installments, each already allocated to an invoice.
    pub installments: Vec<InstallmentCharge>,
    /// The Early Payments stored for the card's invoices.
    pub payments: Vec<PaymentFact>,
}

impl CardFacts {
    /// The dates and state of the invoice closing in `month`, whether or not
    /// it has anything in it.
    pub fn cycle_of(&self, month: Month, today: NaiveDate) -> InvoiceCycle {
        self.cycle(month, today)
    }

    /// Splits a purchase of `total_cents` made on `date` in `count`
    /// Installments and gives each to a consecutive invoice, the first to the
    /// one that would take the purchase paid in full. `None` if the total
    /// cannot be split (see [`split_installments`]).
    pub fn plan_installments(
        &self,
        date: NaiveDate,
        total_cents: i64,
        count: u32,
        today: NaiveDate,
    ) -> Option<Vec<PlannedInstallment>> {
        let first = self.schedule.invoice_month_for(date);
        Some(
            split_installments(total_cents, count)?
                .into_iter()
                .zip(0..)
                .map(|(amount_cents, offset)| PlannedInstallment {
                    number: offset + 1,
                    amount_cents,
                    cycle: self.cycle(first.plus(offset), today),
                })
                .collect(),
        )
    }

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

    /// What the invoice closing in `month` adds up to: its single purchases
    /// and its Installments.
    fn total_of(&self, month: Month) -> i64 {
        let purchases: i64 = self
            .spending
            .iter()
            .filter(|spend| self.schedule.invoice_month_for(spend.date) == month)
            .map(|spend| spend.amount_cents)
            .sum();
        let installments: i64 = self
            .installments
            .iter()
            .filter(|charge| charge.invoice_month == month)
            .map(|charge| charge.amount_cents)
            .sum();
        purchases + installments
    }

    fn early_paid_of(&self, month: Month) -> i64 {
        self.payments
            .iter()
            .filter(|payment| payment.closing_month == month)
            .map(|payment| payment.amount_cents)
            .sum()
    }

    /// The closing months with a purchase, an Installment or a payment,
    /// oldest first.
    fn active_months(&self) -> Vec<Month> {
        let mut months: Vec<Month> = self
            .spending
            .iter()
            .map(|spend| self.schedule.invoice_month_for(spend.date))
            .chain(self.installments.iter().map(|charge| charge.invoice_month))
            .chain(self.payments.iter().map(|payment| payment.closing_month))
            .collect();
        months.sort();
        months.dedup();
        months
    }

    fn invoice(&self, month: Month, today: NaiveDate) -> Invoice {
        let cycle = self.cycle(month, today);
        let total_cents = self.total_of(month);
        let early = self.early_paid_of(month);
        let unpaid = (total_cents - early).max(0);
        let (paid_cents, remaining_cents) = match cycle.state {
            // The due date debits the rest on its own.
            InvoiceState::Paid => (early + unpaid, 0),
            _ => (early, unpaid),
        };
        Invoice {
            card_id: self.id,
            cycle,
            total_cents,
            paid_cents,
            remaining_cents,
        }
    }

    /// The card's invoices, oldest first: every one that has purchases or
    /// payments and the first open one, even when still empty.
    pub fn invoices(&self, today: NaiveDate) -> Vec<Invoice> {
        let mut months = self.active_months();
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

    /// Every payment of the card's invoices, oldest first: the stored Early
    /// Payments and, for each invoice, the automatic payment dated on its due
    /// date for whatever the Early Payments left unpaid. It does not depend on
    /// "today": an automatic payment in the future is a projection of the same
    /// rule, and it changes as purchases are added.
    pub fn payments(&self) -> Vec<InvoicePayment> {
        let mut found = Vec::new();
        for month in self.active_months() {
            let early = self.early_paid_of(month);
            found.extend(
                self.payments
                    .iter()
                    .filter(|payment| payment.closing_month == month)
                    .map(|payment| InvoicePayment {
                        card_id: self.id,
                        closing_month: month,
                        date: payment.date,
                        amount_cents: payment.amount_cents,
                        origin: PaymentOrigin::Early(payment.id),
                    }),
            );
            let unpaid = self.total_of(month) - early;
            if unpaid > 0 {
                found.push(InvoicePayment {
                    card_id: self.id,
                    closing_month: month,
                    date: self.schedule.due_date(month),
                    amount_cents: unpaid,
                    origin: PaymentOrigin::Automatic,
                });
            }
        }
        found.sort_by_key(|payment| (payment.date, payment.closing_month));
        found
    }
}
