//! Monthly Budgets and the spending they are compared with (UC12, UC13).
//!
//! A Budget is a limit for an expense Category that applies every month until
//! the user changes it. The spending of a Category in a Reference Month is the
//! sum of the expenses whose Reference Month is that month:
//!
//! - expenses outside the card, by their own date;
//! - Occurrences of recurring expenses, Projections of the month included;
//! - card purchases and Installments (and Charges), by the month of the due
//!   date of the invoice they are in.
//!
//! The Carried Balance and Invoice Payments are not expenses: the spending was
//! counted when it was made, so they never come in here.
//!
//! The Budget Alert is not stored (ADR-0002): [`Ledger::budget_status`]
//! derives it on every read. Like the rest of the ledger this module knows
//! neither HTTP nor SQL; repositories feed it [`BudgetFacts`].

use std::collections::HashMap;

use chrono::NaiveDate;

use super::{FlowKind, Ledger, cards::Month, last_day_of_month};
use crate::ids::{CategoryId, CreditCardId, RecurringTransactionId};

/// An expense outside the credit card, added up per Category and day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CashExpense {
    pub category_id: CategoryId,
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// Card purchases of one Category on one day, added up. The invoice they enter
/// follows from the day and the card's closing and due days.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardPurchaseExpense {
    pub card_id: CreditCardId,
    pub category_id: CategoryId,
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// An expense that is allocated to an invoice when it is stored: an
/// Installment, and later a Charge. It counts in the month of the invoice's
/// due date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvoiceExpense {
    pub card_id: CreditCardId,
    pub category_id: CategoryId,
    /// The closing month of the invoice it is in.
    pub invoice_month: Month,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// What is stored about expenses, with the Category of each, that the ledger
/// needs to add up the spending of a Category. The invoices themselves come
/// from the cards of [`super::LedgerFacts`].
#[derive(Clone, Debug, Default)]
pub struct SpendingFacts {
    pub cash_expenses: Vec<CashExpense>,
    /// The Category of each recurring expense, to give its Occurrences one.
    pub recurring_categories: HashMap<RecurringTransactionId, CategoryId>,
    pub card_purchases: Vec<CardPurchaseExpense>,
    pub invoice_expenses: Vec<InvoiceExpense>,
}

/// The limit a Budget has from one month on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitVersion {
    pub effective_from: Month,
    /// Cents, greater than zero; `None` if the Budget was removed from this
    /// month on.
    pub limit_cents: Option<i64>,
}

/// The Budget of one Category, with the history of its limit.
#[derive(Clone, Debug)]
pub struct Budget {
    pub category_id: CategoryId,
    /// A deactivated Category keeps its Budget but produces no Alert.
    pub category_active: bool,
    pub versions: Vec<LimitVersion>,
}

impl Budget {
    /// The limit in force in `month` and the month it started in: the latest
    /// version that started on or before it. `None` before the first version
    /// (the Budget did not exist yet) or after a removal.
    pub fn limit_in(&self, month: Month) -> Option<(i64, Month)> {
        let version = self
            .versions
            .iter()
            .filter(|version| version.effective_from <= month)
            .max_by_key(|version| version.effective_from)?;
        Some((version.limit_cents?, version.effective_from))
    }
}

/// What the ledger needs to answer budget questions.
#[derive(Clone, Debug, Default)]
pub struct BudgetFacts {
    pub spending: SpendingFacts,
    pub budgets: Vec<Budget>,
}

/// How a Category stands against its Budget in one Reference Month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetStatus {
    pub category_id: CategoryId,
    pub category_active: bool,
    pub limit_cents: i64,
    pub spent_cents: i64,
    /// The spending as a whole percentage of the limit, rounded down; it
    /// passes 100 when the limit is exceeded.
    pub percent_used: i64,
    /// The Budget Alert: the spending is above the limit and the Category is
    /// active. A deactivated Category never has one.
    pub exceeded: bool,
}

impl Ledger {
    /// Gives the ledger the facts it needs for budget questions.
    pub fn with_budget_facts(mut self, facts: BudgetFacts) -> Self {
        self.budget = facts;
        self
    }

    /// The spending of every Category that has any in the Reference Month
    /// `month`, in cents. See the module documentation for what counts.
    pub fn category_spending(&self, month: Month) -> HashMap<CategoryId, i64> {
        let spending = &self.budget.spending;
        let mut totals: HashMap<CategoryId, i64> = HashMap::new();
        let mut add = |category_id: CategoryId, amount_cents: i64| {
            *totals.entry(category_id).or_default() += amount_cents;
        };

        for expense in &spending.cash_expenses {
            if Month::of(expense.date) == month {
                add(expense.category_id, expense.amount_cents);
            }
        }

        let first_day = month.first_day();
        for occurrence in self.occurrences(first_day, last_day_of_month(first_day)) {
            if occurrence.kind != FlowKind::Expense {
                continue;
            }
            if let Some(category_id) = spending.recurring_categories.get(&occurrence.recurring_id) {
                add(*category_id, occurrence.amount_cents);
            }
        }

        for purchase in &spending.card_purchases {
            if self.reference_month_of_purchase(purchase.card_id, purchase.date) == Some(month) {
                add(purchase.category_id, purchase.amount_cents);
            }
        }

        for expense in &spending.invoice_expenses {
            if self.reference_month_of_invoice(expense.card_id, expense.invoice_month)
                == Some(month)
            {
                add(expense.category_id, expense.amount_cents);
            }
        }
        totals
    }

    /// The Reference Month of a card purchase made on `date`: the month of the
    /// due date of the invoice it enters. `None` for an unknown card.
    fn reference_month_of_purchase(&self, card_id: CreditCardId, date: NaiveDate) -> Option<Month> {
        let schedule = &self.card(card_id)?.schedule;
        Some(Month::of(
            schedule.due_date(schedule.invoice_month_for(date)),
        ))
    }

    /// The Reference Month of everything in the invoice closing in
    /// `invoice_month`. `None` for an unknown card.
    fn reference_month_of_invoice(
        &self,
        card_id: CreditCardId,
        invoice_month: Month,
    ) -> Option<Month> {
        Some(Month::of(
            self.card(card_id)?.schedule.due_date(invoice_month),
        ))
    }

    /// Where every Category with a Budget in `month` stands, in the order the
    /// budgets were given. A Category has no line in a month where it has no
    /// Budget (before it was defined, or after it was removed).
    pub fn budget_status(&self, month: Month) -> Vec<BudgetStatus> {
        let spent = self.category_spending(month);
        self.budget
            .budgets
            .iter()
            .filter_map(|budget| {
                let (limit_cents, _) = budget.limit_in(month)?;
                let spent_cents = spent.get(&budget.category_id).copied().unwrap_or(0);
                Some(BudgetStatus {
                    category_id: budget.category_id,
                    category_active: budget.category_active,
                    limit_cents,
                    spent_cents,
                    percent_used: percent_of(spent_cents, limit_cents),
                    exceeded: budget.category_active && spent_cents > limit_cents,
                })
            })
            .collect()
    }
}

/// `part` as a whole percentage of `whole`, rounded down.
fn percent_of(part: i64, whole: i64) -> i64 {
    (i128::from(part) * 100 / i128::from(whole)) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{LedgerFacts, cards::*};
    use uuid::Uuid;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn month(year: i32, month: u32) -> Month {
        Month::new(year, month).unwrap()
    }

    fn category() -> CategoryId {
        CategoryId(Uuid::new_v4())
    }

    #[test]
    fn a_limit_applies_from_its_month_until_changed_or_removed() {
        let budget = Budget {
            category_id: category(),
            category_active: true,
            versions: vec![
                LimitVersion {
                    effective_from: month(2026, 3),
                    limit_cents: Some(500),
                },
                LimitVersion {
                    effective_from: month(2026, 5),
                    limit_cents: Some(800),
                },
                LimitVersion {
                    effective_from: month(2026, 8),
                    limit_cents: None,
                },
            ],
        };
        assert_eq!(budget.limit_in(month(2026, 2)), None);
        assert_eq!(budget.limit_in(month(2026, 3)), Some((500, month(2026, 3))));
        assert_eq!(budget.limit_in(month(2026, 4)), Some((500, month(2026, 3))));
        assert_eq!(budget.limit_in(month(2026, 5)), Some((800, month(2026, 5))));
        assert_eq!(budget.limit_in(month(2026, 7)), Some((800, month(2026, 5))));
        assert_eq!(budget.limit_in(month(2026, 8)), None);
        assert_eq!(budget.limit_in(month(2027, 1)), None);
    }

    #[test]
    fn percent_rounds_down_and_the_alert_needs_spending_above_the_limit() {
        let food = category();
        let facts = BudgetFacts {
            spending: SpendingFacts {
                cash_expenses: vec![CashExpense {
                    category_id: food,
                    date: day(2026, 1, 10),
                    amount_cents: 1001,
                }],
                ..SpendingFacts::default()
            },
            budgets: vec![Budget {
                category_id: food,
                category_active: true,
                versions: vec![LimitVersion {
                    effective_from: month(2026, 1),
                    limit_cents: Some(1000),
                }],
            }],
        };
        let ledger = Ledger::new(LedgerFacts::default()).with_budget_facts(facts);
        let status = ledger.budget_status(month(2026, 1));
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].percent_used, 100);
        assert!(
            status[0].exceeded,
            "1001 is above 1000 even if it rounds to 100%"
        );
    }

    #[test]
    fn card_expenses_count_in_the_due_month_of_their_invoice() {
        let card = CreditCardId(Uuid::new_v4());
        let food = category();
        // Closes on the 20th, due on the 27th.
        let schedule = CardSchedule::new(vec![ScheduleVersion {
            effective_from: month(2020, 1),
            closing_day: 20,
            due_day: 27,
        }])
        .unwrap();
        let facts = LedgerFacts {
            cards: vec![CardFacts {
                id: card,
                schedule,
                spending: vec![],
                installments: vec![],
                payments: vec![],
                charges: vec![],
                corrections: vec![],
            }],
            ..LedgerFacts::default()
        };
        let spending = SpendingFacts {
            card_purchases: vec![
                CardPurchaseExpense {
                    card_id: card,
                    category_id: food,
                    date: day(2026, 1, 20),
                    amount_cents: 100,
                },
                CardPurchaseExpense {
                    card_id: card,
                    category_id: food,
                    date: day(2026, 1, 21),
                    amount_cents: 10,
                },
            ],
            invoice_expenses: vec![InvoiceExpense {
                card_id: card,
                category_id: food,
                invoice_month: month(2026, 2),
                amount_cents: 1,
            }],
            ..SpendingFacts::default()
        };
        let ledger = Ledger::new(facts).with_budget_facts(BudgetFacts {
            spending,
            budgets: vec![],
        });
        assert_eq!(ledger.category_spending(month(2026, 1))[&food], 100);
        // The purchase after the closing day and the Installment in the
        // invoice closing in February are both due in February.
        assert_eq!(ledger.category_spending(month(2026, 2))[&food], 11);
        assert!(ledger.category_spending(month(2026, 3)).is_empty());
    }

    #[test]
    fn a_due_day_before_the_closing_day_moves_the_reference_month_to_the_next() {
        let card = CreditCardId(Uuid::new_v4());
        let food = category();
        // Closes on the 25th, due on the 5th: the invoice closing in January
        // is due on February 5th.
        let schedule = CardSchedule::new(vec![ScheduleVersion {
            effective_from: month(2020, 1),
            closing_day: 25,
            due_day: 5,
        }])
        .unwrap();
        let ledger = Ledger::new(LedgerFacts {
            cards: vec![CardFacts {
                id: card,
                schedule,
                spending: vec![],
                installments: vec![],
                payments: vec![],
                charges: vec![],
                corrections: vec![],
            }],
            ..LedgerFacts::default()
        })
        .with_budget_facts(BudgetFacts {
            spending: SpendingFacts {
                card_purchases: vec![CardPurchaseExpense {
                    card_id: card,
                    category_id: food,
                    date: day(2026, 1, 10),
                    amount_cents: 100,
                }],
                ..SpendingFacts::default()
            },
            budgets: vec![],
        });
        assert!(ledger.category_spending(month(2026, 1)).is_empty());
        assert_eq!(ledger.category_spending(month(2026, 2))[&food], 100);
    }
}
