//! Reports by Reference Month (UC11): expenses by Category for a period, and
//! income and expenses per month.
//!
//! The expenses and the month each one counts in come from the same place the
//! Budgets use, [`Ledger::expense_entries`] (see [`super::budget`]): outside
//! the card by their own date, Occurrences of recurring expenses (Projections
//! included), card purchases, Installments (an anticipated one is in the open
//! invoice, with its amount already less the Discount) and Charges by the due
//! date of their invoice. Income counts the same way, outside the card by its
//! own date and as Occurrences of recurring incomes.
//!
//! Invoice Payments and the Carried Balance are not here: they are not
//! income or expense, the spending was counted when it was made.
//!
//! Like the rest of the ledger it is pure and nothing is stored (ADR-0002). It
//! reads the expense facts given with [`Ledger::with_budget_facts`] and the
//! incomes of [`super::LedgerFacts`].

use std::collections::HashMap;

use chrono::NaiveDate;

use super::{FlowKind, Ledger, cards::Month, last_day_of_month};
use crate::ids::CategoryId;

/// The longest period a report covers, in months.
pub const MAX_REPORT_MONTHS: u32 = 36;

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

/// One income or expense of a report, in its Reference Month.
struct Entry {
    month: Month,
    kind: FlowKind,
    /// Cents, greater than zero.
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
        from: Month,
        to: Month,
        today: NaiveDate,
    ) -> Result<Vec<CategorySpending>, ReportPeriodError> {
        period_months(from, to)?;
        let mut by_category: HashMap<CategoryId, CategorySpending> = HashMap::new();
        for entry in self.expense_entries(from, to) {
            let spending = by_category
                .entry(entry.category_id)
                .or_insert(CategorySpending {
                    category_id: entry.category_id,
                    total_cents: 0,
                    projected_cents: 0,
                });
            spending.total_cents += entry.amount_cents;
            if entry.date > today {
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
        for entry in self.report_entries(from, to, today) {
            // `report_entries` only gives months of the period.
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
    fn report_entries(&self, from: Month, to: Month, today: NaiveDate) -> Vec<Entry> {
        let in_period = |month: Month| month >= from && month <= to;
        let mut entries: Vec<Entry> = self
            .expense_entries(from, to)
            .into_iter()
            .map(|expense| Entry {
                month: expense.month,
                kind: FlowKind::Expense,
                amount_cents: expense.amount_cents,
                projected: expense.date > today,
            })
            .collect();

        for flow in &self.facts.cash_flows {
            let month = Month::of(flow.date);
            if flow.kind == FlowKind::Income && in_period(month) {
                entries.push(Entry {
                    month,
                    kind: FlowKind::Income,
                    amount_cents: flow.amount_cents,
                    projected: flow.date > today,
                });
            }
        }
        for occurrence in self.occurrences(from.first_day(), last_day_of_month(to.first_day())) {
            if occurrence.kind == FlowKind::Income {
                entries.push(Entry {
                    month: Month::of(occurrence.date),
                    kind: FlowKind::Income,
                    amount_cents: occurrence.amount_cents,
                    projected: occurrence.date > today,
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
