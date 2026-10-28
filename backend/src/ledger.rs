//! The ledger: the calculation module for everything that depends on "today".
//!
//! It is a pure module. It knows neither HTTP nor SQL: repositories feed it
//! the facts stored for one user ([`LedgerFacts`]) and a use case asks it a
//! question with a "today" taken from the injectable clock. Nothing it
//! computes is stored (ADR-0002); it is derived again on every read.
//!
//! Today it answers one question, [`Ledger::current_balance`]. Later tickets
//! add facts (recurring Transactions, card invoices, budgets) and questions
//! (projected balance, invoice state, reports) to this same module, so the
//! rules that depend on dates stay in one place.

use chrono::NaiveDate;

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
}

/// Answers balance questions about one user's [`LedgerFacts`].
#[derive(Clone, Debug)]
pub struct Ledger {
    facts: LedgerFacts,
}

impl Ledger {
    pub fn new(facts: LedgerFacts) -> Self {
        Self { facts }
    }

    /// The Current Balance on `today`, in cents: the Opening Balance plus
    /// incomes minus expenses dated `today` or earlier. Flows dated after
    /// `today` are Projections and stay out until their day arrives. The
    /// result may be negative.
    pub fn current_balance(&self, today: NaiveDate) -> i64 {
        let flows: i64 = self
            .facts
            .cash_flows
            .iter()
            .filter(|flow| flow.date <= today)
            .map(|flow| match flow.kind {
                FlowKind::Income => flow.amount_cents,
                FlowKind::Expense => -flow.amount_cents,
            })
            .sum();
        self.facts.opening_balance_cents.unwrap_or(0) + flows
    }
}
