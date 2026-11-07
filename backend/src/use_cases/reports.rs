//! Reports (UC11): expenses by Category for a period and income x expenses
//! per month, both by Reference Month. Each use case feeds the ledger with
//! what is stored and asks it the question. The balance history, the third
//! report, is [`super::GetBalanceHistoryUseCase`].

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::{
    ids::{CategoryId, UserId},
    ledger::{
        Ledger, LedgerFacts,
        cards::Month,
        reports::{MonthTotals, ReportFacts, ReportPeriodError},
    },
    repositories::{
        card_purchases::CardPurchaseRepository, categories::CategoryRepository,
        credit_cards::CreditCardRepository, recurring::RecurringRepository,
        reports::ReportRepository,
    },
};

use super::credit_card::card_facts;

/// How many months the income x expenses series shows when the caller gives no
/// start: the current month and the five before it.
pub const DEFAULT_SERIES_MONTHS: u32 = 6;

/// Why a report use case failed. Unaware of HTTP: the handler layer decides
/// the status code.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    /// The period breaks a rule. `field` is the request field and `rule` a
    /// stable code for it.
    #[error("invalid period: {field} {rule}")]
    InvalidPeriod {
        field: &'static str,
        rule: &'static str,
    },
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl ReportError {
    fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<ReportPeriodError> for ReportError {
    fn from(error: ReportPeriodError) -> Self {
        match error {
            ReportPeriodError::Order => Self::InvalidPeriod {
                field: "to",
                rule: "period_order",
            },
            ReportPeriodError::TooLong => Self::InvalidPeriod {
                field: "from",
                rule: "period_too_long",
            },
        }
    }
}

/// The months of a report as the caller asked for them; `None` takes the
/// default of the report.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReportPeriod {
    pub from: Option<Month>,
    pub to: Option<Month>,
}

/// Reads what the ledger needs to answer a report question.
struct ReportLoader {
    categories: CategoryRepository,
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
    recurring: RecurringRepository,
    reports: ReportRepository,
}

impl ReportLoader {
    async fn load(&self, user_id: UserId) -> Result<(Ledger, ReportFacts), ReportError> {
        let (_, cards) = card_facts(&self.cards, &self.purchases, user_id)
            .await
            .map_err(ReportError::internal)?;
        let recurring = self
            .recurring
            .list(user_id)
            .await
            .map_err(ReportError::internal)?;
        let recurring_categories = recurring
            .iter()
            .map(|recurring| (recurring.id, recurring.category_id))
            .collect();
        let definitions = recurring
            .into_iter()
            .map(|recurring| recurring.definition)
            .collect();
        let facts = ReportFacts {
            cash_flows: self
                .reports
                .cash_flows(user_id)
                .await
                .map_err(ReportError::internal)?,
            recurring_categories,
            card_purchases: self
                .reports
                .card_purchases(user_id)
                .await
                .map_err(ReportError::internal)?,
            invoice_items: self
                .reports
                .invoice_items(user_id)
                .await
                .map_err(ReportError::internal)?,
        };
        let ledger = Ledger::new(LedgerFacts {
            cards,
            recurring: definitions,
            ..LedgerFacts::default()
        });
        Ok((ledger, facts))
    }
}

macro_rules! constructor {
    () => {
        pub fn new(
            categories: CategoryRepository,
            cards: CreditCardRepository,
            purchases: CardPurchaseRepository,
            recurring: RecurringRepository,
            reports: ReportRepository,
        ) -> Self {
            Self {
                loader: ReportLoader {
                    categories,
                    cards,
                    purchases,
                    recurring,
                    reports,
                },
            }
        }
    };
}

/// What one Category spent in the period, with its name.
#[derive(Clone, Debug)]
pub struct CategoryLine {
    pub category_id: CategoryId,
    pub name: String,
    /// A deactivated Category stays in the report of the months it was used.
    pub active: bool,
    pub total_cents: i64,
    pub projected_cents: i64,
}

/// Expenses by Category, with the period it actually covers.
#[derive(Clone, Debug)]
pub struct ExpensesByCategory {
    pub from: Month,
    pub to: Month,
    pub total_cents: i64,
    pub projected_cents: i64,
    /// The biggest first.
    pub categories: Vec<CategoryLine>,
}

/// UC11 (a): expenses by Category for the Reference Months of a period
/// (default: the current month).
pub struct GetExpensesByCategoryUseCase {
    loader: ReportLoader,
}

impl GetExpensesByCategoryUseCase {
    constructor!();

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
        period: ReportPeriod,
    ) -> Result<ExpensesByCategory, ReportError> {
        let to = period.to.unwrap_or_else(|| Month::of(today));
        let from = period.from.unwrap_or(to);
        let (ledger, facts) = self.loader.load(user_id).await?;
        let spending = ledger.expenses_by_category(&facts, from, to, today)?;

        let names: HashMap<CategoryId, (String, bool)> = self
            .loader
            .categories
            .list(user_id, None, None)
            .await
            .map_err(ReportError::internal)?
            .into_iter()
            .map(|category| (category.id, (category.name, category.active)))
            .collect();
        let mut categories: Vec<CategoryLine> = spending
            .iter()
            .filter_map(|spent| {
                let (name, active) = names.get(&spent.category_id)?.clone();
                Some(CategoryLine {
                    category_id: spent.category_id,
                    name,
                    active,
                    total_cents: spent.total_cents,
                    projected_cents: spent.projected_cents,
                })
            })
            .collect();
        // The ledger orders by amount; ties read better by name.
        categories.sort_by(|a, b| {
            b.total_cents
                .cmp(&a.total_cents)
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(ExpensesByCategory {
            from,
            to,
            total_cents: categories.iter().map(|line| line.total_cents).sum(),
            projected_cents: categories.iter().map(|line| line.projected_cents).sum(),
            categories,
        })
    }
}

/// Income x expenses per month, with the period it actually covers.
#[derive(Clone, Debug)]
pub struct IncomeVsExpenses {
    pub from: Month,
    pub to: Month,
    pub income_cents: i64,
    pub expense_cents: i64,
    /// One entry per month, oldest first.
    pub months: Vec<MonthTotals>,
}

/// UC11 (b): income and expenses of each Reference Month of a period
/// (default: the last six months, this one included).
pub struct GetIncomeVsExpensesUseCase {
    loader: ReportLoader,
}

impl GetIncomeVsExpensesUseCase {
    constructor!();

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
        period: ReportPeriod,
    ) -> Result<IncomeVsExpenses, ReportError> {
        let to = period.to.unwrap_or_else(|| Month::of(today));
        let from = period
            .from
            .unwrap_or_else(|| shift_back(to, DEFAULT_SERIES_MONTHS - 1));
        let (ledger, facts) = self.loader.load(user_id).await?;
        let months = ledger.income_vs_expenses(&facts, from, to, today)?;
        Ok(IncomeVsExpenses {
            from,
            to,
            income_cents: months.iter().map(|month| month.income_cents).sum(),
            expense_cents: months.iter().map(|month| month.expense_cents).sum(),
            months,
        })
    }
}

/// The month `months` before `month`.
fn shift_back(month: Month, months: u32) -> Month {
    (0..months).fold(month, |current, _| current.prev())
}
