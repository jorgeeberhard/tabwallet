//! Reports by Reference Month (UC11): expenses by Category for a period, and
//! income and expenses per month.
//!
//! What a report counts is the same everywhere: the Transactions that are
//! income or expense, each in its Reference Month.
//!
//! - Transactions outside the card count in the month of their own date.
//! - Occurrences of recurring Transactions count in the month they fall in,
//!   Projections included.
//! - Card purchases count in the month of the due date of the invoice they
//!   enter; Installments and Charges in the month of the due date of the
//!   invoice they are in (an anticipated Installment is in the open invoice,
//!   with its amount already less the Discount).
//!
//! Invoice Payments and the Carried Balance are not here: they are not
//! income or expense, the spending was counted when it was made.
//!
//! Like the rest of the ledger it is pure: repositories feed it
//! [`ReportFacts`], which carries the Category of each item, and nothing is
//! stored (ADR-0002).

use std::collections::HashMap;

use chrono::NaiveDate;

use super::{FlowKind, Ledger, cards::Month, last_day_of_month};
use crate::ids::{CategoryId, CreditCardId, RecurringTransactionId};

/// The longest period a report covers, in months.
pub const MAX_REPORT_MONTHS: u32 = 36;

/// Income or expense outside the credit card of one Category on one day,
/// added up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportCashFlow {
    pub category_id: CategoryId,
    pub kind: FlowKind,
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// Card purchases of one Category on one day, added up. The invoice they enter
/// follows from the day and the card's closing and due days.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportCardPurchase {
    pub card_id: CreditCardId,
    pub category_id: CategoryId,
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// An expense that is in an invoice by the invoice itself, not by its date: an
/// Installment or a Charge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportInvoiceItem {
    pub card_id: CreditCardId,
    pub category_id: CategoryId,
    /// The closing month of the invoice it is in.
    pub invoice_month: Month,
    /// The day the item was made: the date of the purchase an Installment
    /// belongs to, or of the Charge. It decides whether the item is still a
    /// Projection; the month it counts in does not depend on it.
    pub date: NaiveDate,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// What is stored, with the Category of each item, that the reports need. The
/// invoices themselves come from the cards of [`super::LedgerFacts`].
#[derive(Clone, Debug, Default)]
pub struct ReportFacts {
    pub cash_flows: Vec<ReportCashFlow>,
    /// The Category of each recurring Transaction, to give its Occurrences one.
    pub recurring_categories: HashMap<RecurringTransactionId, CategoryId>,
    pub card_purchases: Vec<ReportCardPurchase>,
    /// Installments and Charges.
    pub invoice_items: Vec<ReportInvoiceItem>,
}

/// Why a report period was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportPeriodError {
    /// The first month is after the last.
    Order,
    /// The period has more than [`MAX_REPORT_MONTHS`] months.
    TooLong,
}

/// What one Category spent in a period.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CategorySpending {
    pub category_id: CategoryId,
    pub total_cents: i64,
    /// The part of the total that is a Projection: its date is after today.
    pub projected_cents: i64,
}

/// Income and expenses of one Reference Month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthTotals {
    pub month: Month,
    pub income_cents: i64,
    pub expense_cents: i64,
    pub projected_income_cents: i64,
    pub projected_expense_cents: i64,
}

/// One item of a report, in its Reference Month.
struct Entry {
    month: Month,
    kind: FlowKind,
    category_id: CategoryId,
    amount_cents: i64,
    /// Its date is after today.
    projected: bool,
}

/// Checks a period of whole months and lists them, oldest first.
fn period_months(from: Month, to: Month) -> Result<Vec<Month>, ReportPeriodError> {
    if from > to {
        return Err(ReportPeriodError::Order);
    }
    let mut months = vec![from];
    while let Some(&last) = months.last()
        && last < to
    {
        if months.len() == MAX_REPORT_MONTHS as usize {
            return Err(ReportPeriodError::TooLong);
        }
        months.push(last.next());
    }
    Ok(months)
}

impl Ledger {
    /// What each Category spent in the Reference Months from `from` to `to`,
    /// both inclusive, the biggest first. Categories with no expense in the
    /// period are left out. See the module documentation for what counts.
    pub fn expenses_by_category(
        &self,
        facts: &ReportFacts,
        from: Month,
        to: Month,
        today: NaiveDate,
    ) -> Result<Vec<CategorySpending>, ReportPeriodError> {
        period_months(from, to)?;
        let mut by_category: HashMap<CategoryId, CategorySpending> = HashMap::new();
        for entry in self.report_entries(facts, from, to, today) {
            if entry.kind != FlowKind::Expense {
                continue;
            }
            let spending = by_category
                .entry(entry.category_id)
                .or_insert(CategorySpending {
                    category_id: entry.category_id,
                    total_cents: 0,
                    projected_cents: 0,
                });
            spending.total_cents += entry.amount_cents;
            if entry.projected {
                spending.projected_cents += entry.amount_cents;
            }
        }
        let mut found: Vec<CategorySpending> = by_category.into_values().collect();
        found.sort_by_key(|spending| {
            (
                std::cmp::Reverse(spending.total_cents),
                spending.category_id.0,
            )
        });
        Ok(found)
    }

    /// Income and expenses of each Reference Month from `from` to `to`, both
    /// inclusive, oldest first, with a row for every month even if empty.
    pub fn income_vs_expenses(
        &self,
        facts: &ReportFacts,
        from: Month,
        to: Month,
        today: NaiveDate,
    ) -> Result<Vec<MonthTotals>, ReportPeriodError> {
        let mut rows: Vec<MonthTotals> = period_months(from, to)?
            .into_iter()
            .map(|month| MonthTotals {
                month,
                income_cents: 0,
                expense_cents: 0,
                projected_income_cents: 0,
                projected_expense_cents: 0,
            })
            .collect();
        for entry in self.report_entries(facts, from, to, today) {
            // `report_entries` only gives months of the period, in order.
            let Some(row) = rows.iter_mut().find(|row| row.month == entry.month) else {
                continue;
            };
            let (total, projected) = match entry.kind {
                FlowKind::Income => (&mut row.income_cents, &mut row.projected_income_cents),
                FlowKind::Expense => (&mut row.expense_cents, &mut row.projected_expense_cents),
            };
            *total += entry.amount_cents;
            if entry.projected {
                *projected += entry.amount_cents;
            }
        }
        Ok(rows)
    }

    /// Every income and expense whose Reference Month is from `from` to `to`.
    fn report_entries(
        &self,
        facts: &ReportFacts,
        from: Month,
        to: Month,
        today: NaiveDate,
    ) -> Vec<Entry> {
        let in_period = |month: Month| month >= from && month <= to;
        let mut entries = Vec::new();

        for flow in &facts.cash_flows {
            let month = Month::of(flow.date);
            if in_period(month) {
                entries.push(Entry {
                    month,
                    kind: flow.kind,
                    category_id: flow.category_id,
                    amount_cents: flow.amount_cents,
                    projected: flow.date > today,
                });
            }
        }

        for occurrence in self.occurrences(from.first_day(), last_day_of_month(to.first_day())) {
            if let Some(category_id) = facts.recurring_categories.get(&occurrence.recurring_id) {
                entries.push(Entry {
                    month: Month::of(occurrence.date),
                    kind: occurrence.kind,
                    category_id: *category_id,
                    amount_cents: occurrence.amount_cents,
                    projected: occurrence.date > today,
                });
            }
        }

        for purchase in &facts.card_purchases {
            let Some(card) = self.card(purchase.card_id) else {
                continue;
            };
            let month = Month::of(
                card.schedule
                    .due_date(card.schedule.invoice_month_for(purchase.date)),
            );
            if in_period(month) {
                entries.push(Entry {
                    month,
                    kind: FlowKind::Expense,
                    category_id: purchase.category_id,
                    amount_cents: purchase.amount_cents,
                    projected: purchase.date > today,
                });
            }
        }

        for item in &facts.invoice_items {
            let Some(card) = self.card(item.card_id) else {
                continue;
            };
            let month = Month::of(card.schedule.due_date(item.invoice_month));
            if in_period(month) {
                entries.push(Entry {
                    month,
                    kind: FlowKind::Expense,
                    category_id: item.category_id,
                    amount_cents: item.amount_cents,
                    projected: item.date > today,
                });
            }
        }

        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month(year: i32, month: u32) -> Month {
        Month::new(year, month).unwrap()
    }

    #[test]
    fn a_period_lists_its_months_and_refuses_upside_down_or_too_long_ones() {
        assert_eq!(
            period_months(month(2025, 11), month(2026, 2)).unwrap(),
            vec![
                month(2025, 11),
                month(2025, 12),
                month(2026, 1),
                month(2026, 2)
            ]
        );
        assert_eq!(
            period_months(month(2026, 2), month(2026, 1)),
            Err(ReportPeriodError::Order)
        );
        assert_eq!(
            period_months(month(2023, 2), month(2026, 1)).map(|months| months.len()),
            Ok(36)
        );
        assert_eq!(
            period_months(month(2023, 1), month(2026, 1)),
            Err(ReportPeriodError::TooLong)
        );
    }
}
