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
//! Paying an invoice (ADR-0003): the user may make Early Payments, total
//! or partial, while the invoice is Open or Closed. On the due date the ledger
//! derives an automatic Invoice Payment of whatever is still unpaid and the
//! invoice becomes Paid. Early Payments are stored; the automatic one never
//! is. Neither is an expense: they leave the balance on their date and are
//! reported as [`InvoicePayment`]s, apart from card purchases.
//!
//! Correcting the payment and Charges: the user may replace the
//! due-date payment with the amount really paid; what is left unpaid becomes
//! the Carried Balance of the next invoice, a derived item that is not an
//! expense and that counts in that invoice's total (and so in what it carries
//! on in turn). Charges are expenses the user records in an invoice and are part
//! of its total. Both are stored as facts; the amounts follow from them.
//!
//! An invoice never comes to less than what was paid on it early. A use case
//! that is about to lower a total (editing or deleting a purchase, an
//! Installment or a Charge, new card days, an anticipation, a corrected
//! payment) asks [`CardFacts::after`] and [`CardFacts::overpaid_invoices`]
//! beforehand and refuses the change that would break it.
//!
//! A card keeps a history of its days ([`CardSchedule`]) so that changing them
//! only affects invoices still open: closed and paid invoices keep the days
//! they were closed with.

use std::collections::{BTreeSet, HashMap};

use chrono::NaiveDate;

pub use super::month::Month;
use crate::ids::{CreditCardId, InvoicePaymentId};

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

    /// The schedule after new days start at the invoice closing in `from`,
    /// replacing any version that already starts there or later.
    pub fn with_days_from(&self, from: Month, closing_day: u32, due_day: u32) -> Self {
        let mut versions: Vec<ScheduleVersion> = self
            .versions
            .iter()
            .filter(|version| version.effective_from < from)
            .copied()
            .collect();
        versions.push(ScheduleVersion {
            effective_from: from,
            closing_day,
            due_day,
        });
        Self { versions }
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
    /// The Reference Month of everything in the invoice closing in `month`:
    /// the month of its due date. The one place that rule lives.
    pub fn reference_month(&self, month: Month) -> Month {
        Month::of(self.due_date(month))
    }

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

impl InvoiceState {
    /// The stable name the API uses for the state.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Paid => "paid",
        }
    }
}

/// The dates and state of one invoice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvoiceCycle {
    pub closing_month: Month,
    pub closing_date: NaiveDate,
    pub due_date: NaiveDate,
    /// The month everything in the invoice counts in for reports and budgets:
    /// the month of its due date ([`CardSchedule::reference_month`]).
    pub reference_month: Month,
    pub state: InvoiceState,
}

/// Whether the payment of an invoice can be corrected to the amount really
/// paid at the bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrectionState {
    /// The due date has not come: there is no payment yet to correct.
    NotDue,
    /// Early Payments covered the whole invoice: the due date debits nothing.
    NothingToPay,
    /// The invoice that received (or would receive) the Carried Balance is
    /// Paid, so the payment is locked like the rest of the invoice.
    Locked,
    /// The invoice is Paid and the next one is not: the payment can be
    /// corrected, again and again.
    Editable,
}

impl CorrectionState {
    /// The stable name the API uses for the state.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotDue => "not_due",
            Self::NothingToPay => "nothing_to_pay",
            Self::Locked => "locked",
            Self::Editable => "editable",
        }
    }
}

/// The state of an invoice's payment correction and its numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaymentCorrection {
    pub state: CorrectionState,
    /// The amount the user said they paid, once they corrected it.
    pub corrected_cents: Option<i64>,
    /// The most the due-date payment can be corrected to: what the Early
    /// Payments left unpaid.
    pub max_cents: i64,
}

/// An invoice with its amounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invoice {
    pub card_id: CreditCardId,
    pub cycle: InvoiceCycle,
    /// What the invoice comes to: the purchases, the Charges and the Carried
    /// Balance it received from the previous invoice.
    pub total_cents: i64,
    /// What has left the balance for this invoice: the Early Payments, and
    /// once the due date has come, the due-date payment (automatic, or the
    /// amount the user corrected it to).
    pub paid_cents: i64,
    /// What is still to be paid: the total less the Early Payments, never
    /// negative. Zero once the invoice is Paid (the due date settles it); an
    /// Open or Closed invoice paid in full early also has zero remaining but
    /// keeps its state until the due date, so purchases may still raise it.
    pub remaining_cents: i64,
    /// The sum of the Charges recorded in it (part of `total_cents`).
    pub charges_cents: i64,
    /// The Carried Balance received from the previous invoice (part of
    /// `total_cents`). It is not an expense.
    pub carried_in_cents: i64,
    /// What was left unpaid at the due date and moved to the next invoice as
    /// its Carried Balance.
    pub carried_out_cents: i64,
    pub payment_correction: PaymentCorrection,
}

/// Where a payment of an invoice came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaymentOrigin {
    /// Made by the user before the due date and stored.
    Early(InvoicePaymentId),
    /// Derived on the due date from what was still unpaid; never stored.
    Automatic,
    /// The due-date payment after the user corrected it to the amount really
    /// paid. The correction is stored; the payment is derived from it.
    Corrected,
}

impl PaymentOrigin {
    /// Whether the payment is derived on read rather than a stored row.
    pub fn is_derived(self) -> bool {
        !matches!(self, Self::Early(_))
    }
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

/// A Charge as the ledger needs it: how much, and the invoice it was recorded
/// in (by closing month, not by date: the user links it to the invoice).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChargeFact {
    pub closing_month: Month,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// The amount the user said they really paid of an invoice at its due date
///. It replaces the automatic payment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorrectionFact {
    pub closing_month: Month,
    /// Cents, zero or more.
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

/// A change to the facts of one card that a use case is about to store, so
/// the ledger can say beforehand what it would do to the invoices. Each field
/// is something taken away or added; the rest stays as it is.
#[derive(Clone, Debug, Default)]
pub struct CardChange {
    /// The days the card would have.
    pub schedule: Option<CardSchedule>,
    pub spend_removed: Vec<CardSpend>,
    pub spend_added: Vec<CardSpend>,
    pub installments_removed: Vec<InstallmentCharge>,
    pub installments_added: Vec<InstallmentCharge>,
    pub charges_removed: Vec<ChargeFact>,
    pub charges_added: Vec<ChargeFact>,
    /// The due-date payment of an invoice, as corrected.
    pub correction: Option<CorrectionFact>,
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
    /// The Charges recorded in the card's invoices.
    pub charges: Vec<ChargeFact>,
    /// The corrections of the due-date payment.
    pub corrections: Vec<CorrectionFact>,
}

/// The amounts of one invoice, worked out from the facts alone (they do not
/// depend on "today"). An invoice's unpaid part may become the Carried Balance
/// of the next one, so invoices are settled oldest first.
#[derive(Clone, Copy, Debug)]
struct Settlement {
    month: Month,
    purchases_cents: i64,
    charges_cents: i64,
    carried_in_cents: i64,
    early_cents: i64,
    /// The total less the Early Payments, never negative: what the due date
    /// would debit if the user did not correct the payment.
    unpaid_cents: i64,
    /// The Early Payments came to more than the total.
    overpaid: bool,
    /// What the user said they paid (at most `unpaid_cents`), if corrected.
    corrected_cents: Option<i64>,
    /// What leaves the balance on the due date.
    due_payment_cents: i64,
    /// What moves to the next invoice: `unpaid_cents` less `due_payment_cents`.
    carried_out_cents: i64,
}

/// Takes out of `facts` one element equal to each of `removed`.
fn remove_each<T: PartialEq>(facts: &mut Vec<T>, removed: &[T]) {
    for gone in removed {
        if let Some(index) = facts.iter().position(|fact| fact == gone) {
            facts.remove(index);
        }
    }
}

impl Settlement {
    fn total_cents(&self) -> i64 {
        self.purchases_cents + self.charges_cents + self.carried_in_cents
    }
}

impl CardFacts {
    /// The dates and state of the invoice closing in `month`, whether or not
    /// it has anything in it.
    pub fn cycle_of(&self, month: Month, today: NaiveDate) -> InvoiceCycle {
        self.cycle(month, today)
    }

    /// The facts as they would be with `change` stored.
    pub fn after(&self, change: &CardChange) -> CardFacts {
        let mut facts = self.clone();
        if let Some(schedule) = &change.schedule {
            facts.schedule = schedule.clone();
        }
        for removed in &change.spend_removed {
            if let Some(spend) = facts
                .spending
                .iter_mut()
                .find(|spend| spend.date == removed.date)
            {
                spend.amount_cents -= removed.amount_cents;
            }
        }
        facts.spending.retain(|spend| spend.amount_cents > 0);
        facts.spending.extend(change.spend_added.iter().copied());
        remove_each(&mut facts.installments, &change.installments_removed);
        facts
            .installments
            .extend(change.installments_added.iter().copied());
        remove_each(&mut facts.charges, &change.charges_removed);
        facts.charges.extend(change.charges_added.iter().copied());
        if let Some(correction) = change.correction {
            facts
                .corrections
                .retain(|other| other.closing_month != correction.closing_month);
            facts.corrections.push(correction);
        }
        facts
    }

    /// The invoices whose Early Payments come to more than their total. None
    /// should exist: a change that would create one is refused.
    pub fn overpaid_invoices(&self) -> BTreeSet<Month> {
        self.settlements(None)
            .into_iter()
            .filter(|settlement| settlement.overpaid)
            .map(|settlement| settlement.month)
            .collect()
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
                    cycle: self.cycle(first.plus(offset as i32), today),
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
            reference_month: self.schedule.reference_month(month),
            state,
        }
    }

    /// The dates and state of the invoice a purchase made on `date` enters.
    pub fn cycle_for_purchase(&self, date: NaiveDate, today: NaiveDate) -> InvoiceCycle {
        self.cycle(self.schedule.invoice_month_for(date), today)
    }

    /// What the invoice closing in `month` adds up to from purchases: its
    /// single purchases and its Installments (without Charges or any Carried
    /// Balance).
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

    fn charges_of(&self, month: Month) -> i64 {
        self.charges
            .iter()
            .filter(|charge| charge.closing_month == month)
            .map(|charge| charge.amount_cents)
            .sum()
    }

    fn early_paid_of(&self, month: Month) -> i64 {
        self.payments
            .iter()
            .filter(|payment| payment.closing_month == month)
            .map(|payment| payment.amount_cents)
            .sum()
    }

    /// Works out every invoice that has something in it, oldest first: those
    /// with purchases, Installments, Charges, payments or a correction, and those that
    /// receive a Carried Balance, plus `also` (the first open invoice, which
    /// exists even when empty).
    ///
    /// The unpaid part an invoice's due-date payment leaves (see
    /// [`Settlement::carried_out_cents`]) is the Carried Balance of the next
    /// invoice, which counts in that invoice's total and so in what it carries
    /// on in turn.
    fn settlements(&self, also: Option<Month>) -> Vec<Settlement> {
        let mut pending: BTreeSet<Month> = self
            .spending
            .iter()
            .map(|spend| self.schedule.invoice_month_for(spend.date))
            .chain(self.installments.iter().map(|charge| charge.invoice_month))
            .chain(self.payments.iter().map(|payment| payment.closing_month))
            .chain(self.charges.iter().map(|charge| charge.closing_month))
            .chain(
                self.corrections
                    .iter()
                    .map(|correction| correction.closing_month),
            )
            .chain(also)
            .collect();
        let mut carried_in: HashMap<Month, i64> = HashMap::new();
        let mut found = Vec::new();
        while let Some(month) = pending.pop_first() {
            let carried_in_cents = carried_in.remove(&month).unwrap_or(0);
            let purchases_cents = self.total_of(month);
            let charges_cents = self.charges_of(month);
            let early_cents = self.early_paid_of(month);
            let balance = purchases_cents + charges_cents + carried_in_cents - early_cents;
            let unpaid_cents = balance.max(0);
            let corrected_cents = self
                .corrections
                .iter()
                .find(|correction| correction.closing_month == month)
                .map(|correction| correction.amount_cents.min(unpaid_cents));
            let due_payment_cents = corrected_cents.unwrap_or(unpaid_cents);
            let carried_out_cents = unpaid_cents - due_payment_cents;
            if carried_out_cents > 0 {
                carried_in.insert(month.next(), carried_out_cents);
                pending.insert(month.next());
            }
            found.push(Settlement {
                month,
                purchases_cents,
                charges_cents,
                carried_in_cents,
                early_cents,
                unpaid_cents,
                overpaid: balance < 0,
                corrected_cents,
                due_payment_cents,
                carried_out_cents,
            });
        }
        found
    }

    fn invoice_of(&self, settlement: &Settlement, today: NaiveDate) -> Invoice {
        let cycle = self.cycle(settlement.month, today);
        let (paid_cents, remaining_cents) = match cycle.state {
            // The due date debits what the Early Payments left (or the amount
            // the user corrected it to) on its own.
            InvoiceState::Paid => (settlement.early_cents + settlement.due_payment_cents, 0),
            _ => (settlement.early_cents, settlement.unpaid_cents),
        };
        let state = if cycle.state != InvoiceState::Paid {
            CorrectionState::NotDue
        } else if self.cycle(settlement.month.next(), today).state == InvoiceState::Paid {
            CorrectionState::Locked
        } else if settlement.unpaid_cents == 0 {
            CorrectionState::NothingToPay
        } else {
            CorrectionState::Editable
        };
        Invoice {
            card_id: self.id,
            cycle,
            total_cents: settlement.total_cents(),
            paid_cents,
            remaining_cents,
            charges_cents: settlement.charges_cents,
            carried_in_cents: settlement.carried_in_cents,
            carried_out_cents: settlement.carried_out_cents,
            payment_correction: PaymentCorrection {
                state,
                corrected_cents: settlement.corrected_cents,
                max_cents: settlement.unpaid_cents,
            },
        }
    }

    /// The card's invoices, oldest first: every one that has purchases,
    /// Charges, payments or a Carried Balance, and the first open one, even
    /// when still empty.
    pub fn invoices(&self, today: NaiveDate) -> Vec<Invoice> {
        self.settlements(Some(self.schedule.first_open_month(today)))
            .iter()
            .map(|settlement| self.invoice_of(settlement, today))
            .collect()
    }

    pub fn find_invoice(&self, month: Month, today: NaiveDate) -> Option<Invoice> {
        self.invoices(today)
            .into_iter()
            .find(|invoice| invoice.cycle.closing_month == month)
    }

    /// Every payment of the card's invoices, oldest first: the stored Early
    /// Payments and, for each invoice, the payment dated on its due date for
    /// whatever the Early Payments left unpaid (the automatic payment) or, once
    /// the user corrected it, for the amount they said they paid. It does not
    /// depend on "today": a due-date payment in the future is a projection of
    /// the same rule, and it changes as purchases are added.
    pub fn payments(&self) -> Vec<InvoicePayment> {
        let mut found = Vec::new();
        for settlement in self.settlements(None) {
            let month = settlement.month;
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
            if settlement.due_payment_cents > 0 {
                found.push(InvoicePayment {
                    card_id: self.id,
                    closing_month: month,
                    date: self.schedule.due_date(month),
                    amount_cents: settlement.due_payment_cents,
                    origin: if settlement.corrected_cents.is_some() {
                        PaymentOrigin::Corrected
                    } else {
                        PaymentOrigin::Automatic
                    },
                });
            }
        }
        found.sort_by_key(|payment| (payment.date, payment.closing_month));
        found
    }
}
