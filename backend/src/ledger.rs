//! The ledger: the pure calculation module (no HTTP, no SQL) behind everything
//! that depends on "today". Nothing it computes is stored (ADR-0002).

pub mod anticipation;
pub mod budget;
pub mod cards;
pub mod month;
mod recurring;
pub mod reports;
mod series;

use chrono::NaiveDate;

pub use recurring::{
    AmountFrom, Occurrence, OccurrenceChange, OccurrenceException, RecurringDefinition,
    date_in_month, first_of_month, last_day_of_month, next_month, previous_month,
};

pub use budget::{Budget, BudgetFacts, BudgetStatus, SpendingFacts};

pub use series::{BalancePoint, Granularity, MAX_SERIES_POINTS, SeriesError};

use crate::ids::RecurringTransactionId;

/// How many months after the current one the ledger looks ahead when asked
/// for Projections without an end date (the Projected Balance window).
pub const PROJECTION_HORIZON_MONTHS: u32 = 12;

/// The last day the ledger projects recurring Occurrences to: the end of the
/// month [`PROJECTION_HORIZON_MONTHS`] after the month of `today`.
pub fn projection_horizon(today: NaiveDate) -> NaiveDate {
    Month::of(today)
        .plus(PROJECTION_HORIZON_MONTHS as i32)
        .last_day()
}

use crate::ids::CreditCardId;
use cards::{CardFacts, Invoice, InvoiceCycle, InvoicePayment, Month, PlannedInstallment};

/// Which way money moves in a [`CashFlow`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowKind {
    Income,
    Expense,
}

/// Money that moved, or will move, in or out of the user's balance on one
/// day. Only flows that hit the balance directly belong here (incomes and
/// expenses outside the credit card); card purchases reach it later through
/// invoice payments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CashFlow {
    pub date: NaiveDate,
    pub kind: FlowKind,
    /// Cents, greater than zero: `kind` gives the direction. Several flows of
    /// the same day and kind may arrive already added up.
    pub amount_cents: i64,
}

/// What is stored for one user that the ledger needs.
#[derive(Clone, Debug, Default)]
pub struct LedgerFacts {
    /// The Opening Balance, if the user informed one. It may be zero or
    /// negative.
    pub opening_balance_cents: Option<i64>,
    pub cash_flows: Vec<CashFlow>,
    /// The user's credit cards with their closing and due days, purchases and
    /// Early Payments. Card purchases never enter `cash_flows`: they reach the
    /// balance only through Invoice Payments, which the ledger derives from
    /// these facts.
    pub cards: Vec<CardFacts>,
    /// The user's recurring Transactions. Their Occurrences are derived from
    /// these and never stored.
    pub recurring: Vec<RecurringDefinition>,
}

/// The amount with the sign of its direction: positive in, negative out.
fn signed(kind: FlowKind, amount_cents: i64) -> i64 {
    match kind {
        FlowKind::Income => amount_cents,
        FlowKind::Expense => -amount_cents,
    }
}

/// Answers balance questions about one user's [`LedgerFacts`].
#[derive(Clone, Debug)]
pub struct Ledger {
    facts: LedgerFacts,
    /// What the budget questions need besides `facts` (see [`budget`]).
    budget: BudgetFacts,
}

impl Ledger {
    pub fn new(facts: LedgerFacts) -> Self {
        Self {
            facts,
            budget: BudgetFacts::default(),
        }
    }

    /// A ledger that knows only these recurring Transactions: enough to ask
    /// for their Occurrences.
    pub fn of_recurring(recurring: Vec<RecurringDefinition>) -> Self {
        Self::new(LedgerFacts {
            recurring,
            ..LedgerFacts::default()
        })
    }

    /// A ledger that knows only the cards: enough to ask about invoices.
    pub fn of_cards(cards: Vec<CardFacts>) -> Self {
        Self::new(LedgerFacts {
            cards,
            ..LedgerFacts::default()
        })
    }

    /// The Current Balance on `today`, in cents: the Opening Balance plus
    /// incomes minus expenses dated `today` or earlier, counting the
    /// Occurrences of recurring Transactions that have already fallen, minus
    /// the Invoice Payments (early and automatic) dated `today` or earlier.
    /// Flows dated after `today` are Projections and stay out until their day
    /// arrives. The result may be negative.
    pub fn current_balance(&self, today: NaiveDate) -> i64 {
        let flows: i64 = self
            .facts
            .cash_flows
            .iter()
            .filter(|flow| flow.date <= today)
            .map(|flow| signed(flow.kind, flow.amount_cents))
            .sum();
        let occurrences: i64 = self
            .occurrences(NaiveDate::MIN, today)
            .iter()
            .map(|occurrence| signed(occurrence.kind, occurrence.amount_cents))
            .sum();
        let payments: i64 = self
            .invoice_payments(NaiveDate::MIN, today)
            .iter()
            .map(|payment| payment.amount_cents)
            .sum();
        self.facts.opening_balance_cents.unwrap_or(0) + flows + occurrences - payments
    }

    /// The Invoice Payments of every card dated from `from` to `to`, both
    /// inclusive, oldest first: the stored Early Payments and the automatic
    /// payments that fall on the due dates (see [`cards::CardFacts::payments`]).
    /// Whether one is a Projection (after today) is for the caller to say.
    pub fn invoice_payments(&self, from: NaiveDate, to: NaiveDate) -> Vec<InvoicePayment> {
        let mut found: Vec<InvoicePayment> = self
            .facts
            .cards
            .iter()
            .flat_map(|card| card.payments())
            .filter(|payment| payment.date >= from && payment.date <= to)
            .collect();
        found.sort_by_key(|payment| (payment.date, payment.card_id.0, payment.closing_month));
        found
    }

    /// The payments of one invoice (early and automatic), oldest first.
    pub fn payments_of_invoice(
        &self,
        card_id: CreditCardId,
        closing_month: Month,
    ) -> Vec<InvoicePayment> {
        self.card(card_id)
            .map(|card| {
                card.payments()
                    .into_iter()
                    .filter(|payment| payment.closing_month == closing_month)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The Occurrences of every recurring Transaction that fall on a day from
    /// `from` to `to`, both inclusive, oldest first. Whether one is a
    /// Projection (after today) is for the caller to say.
    pub fn occurrences(&self, from: NaiveDate, to: NaiveDate) -> Vec<Occurrence> {
        let mut found: Vec<Occurrence> = self
            .facts
            .recurring
            .iter()
            .flat_map(|definition| definition.occurrences_between(from, to))
            .collect();
        found.sort_by_key(|occurrence| (occurrence.date, occurrence.recurring_id));
        found
    }

    /// One Occurrence, or `None` if the recurring Transaction has none in that
    /// month (not started, ended or deleted by the user).
    pub fn occurrence(&self, id: RecurringTransactionId, month: NaiveDate) -> Option<Occurrence> {
        self.facts
            .recurring
            .iter()
            .find(|definition| definition.id == id)?
            .occurrence_in(month)
    }

    fn card(&self, card_id: CreditCardId) -> Option<&CardFacts> {
        self.facts.cards.iter().find(|card| card.id == card_id)
    }

    /// The invoices of a card, oldest first: those that have purchases and the
    /// first open one. Empty for an unknown card.
    pub fn invoices(&self, card_id: CreditCardId, today: NaiveDate) -> Vec<Invoice> {
        self.card(card_id)
            .map(|card| card.invoices(today))
            .unwrap_or_default()
    }

    /// One invoice of a card by its closing month, if it exists (see
    /// [`Ledger::invoices`]).
    pub fn invoice(
        &self,
        card_id: CreditCardId,
        closing_month: Month,
        today: NaiveDate,
    ) -> Option<Invoice> {
        self.card(card_id)?.find_invoice(closing_month, today)
    }

    /// The invoice a purchase made on `date` with this card belongs to: a
    /// purchase on or before the closing day enters the invoice closing that
    /// month, a later one the next month's.
    pub fn invoice_for_purchase(
        &self,
        card_id: CreditCardId,
        date: NaiveDate,
        today: NaiveDate,
    ) -> Option<InvoiceCycle> {
        self.card(card_id)
            .map(|card| card.cycle_for_purchase(date, today))
    }

    /// The dates and state of the invoice of a card closing in `closing_month`,
    /// whether or not anything is in it yet. `None` for an unknown card.
    pub fn invoice_cycle(
        &self,
        card_id: CreditCardId,
        closing_month: Month,
        today: NaiveDate,
    ) -> Option<InvoiceCycle> {
        self.card(card_id)
            .map(|card| card.cycle_of(closing_month, today))
    }

    /// How a purchase of `total_cents` made on `date` with this card would be
    /// split in `count` Installments: exact cents with the leftover on the
    /// first, each in a consecutive invoice starting with the one that would
    /// take the purchase paid in full. `None` for an unknown card or a total
    /// that gives some Installment less than one cent.
    pub fn plan_installments(
        &self,
        card_id: CreditCardId,
        date: NaiveDate,
        total_cents: i64,
        count: u32,
        today: NaiveDate,
    ) -> Option<Vec<PlannedInstallment>> {
        self.card(card_id)?
            .plan_installments(date, total_cents, count, today)
    }

    /// The invoice of the card that is open on `today` (the one receiving
    /// purchases), whether or not anything is in it yet. `None` for an
    /// unknown card.
    pub fn open_invoice_cycle(
        &self,
        card_id: CreditCardId,
        today: NaiveDate,
    ) -> Option<InvoiceCycle> {
        let card = self.card(card_id)?;
        Some(card.cycle_of(card.schedule.first_open_month(today), today))
    }

    /// Plans an Installment Anticipation: bringing `chosen` Installments of
    /// the card into its open invoice, with a Discount of `discount_cents`
    /// (zero for none) split in proportion between them. Nothing is stored;
    /// see [`anticipation`] for the rules.
    pub fn plan_anticipation(
        &self,
        card_id: CreditCardId,
        chosen: &[anticipation::InstallmentToAnticipate],
        discount_cents: i64,
        today: NaiveDate,
    ) -> Result<anticipation::AnticipationPlan, anticipation::AnticipationError> {
        self.card(card_id)
            .ok_or(anticipation::AnticipationError::UnknownCard)?
            .plan_anticipation(chosen, discount_cents, today)
    }

    /// The first invoice of the card that `change` would leave with less in it
    /// than was paid on it early (and that is not already in that state), if
    /// any. A use case asks this before storing a change that lowers a total,
    /// and refuses the change when it gets an answer.
    pub fn overpaid_by(&self, card_id: CreditCardId, change: &cards::CardChange) -> Option<Month> {
        let card = self.card(card_id)?;
        let before = card.overpaid_invoices();
        card.after(change)
            .overpaid_invoices()
            .into_iter()
            .find(|month| !before.contains(month))
    }

    /// Whether any invoice of the card still has something to pay on `today`.
    /// A card in this state cannot be deactivated.
    pub fn has_remaining_invoice(&self, card_id: CreditCardId, today: NaiveDate) -> bool {
        self.invoices(card_id, today)
            .iter()
            .any(|invoice| invoice.remaining_cents > 0)
    }
}
