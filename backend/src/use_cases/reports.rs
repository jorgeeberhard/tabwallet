//! Reports (UC11): expenses by Category for a period and income x expenses
//! per month, both by Reference Month. Each use case feeds the ledger with
//! what is stored and asks it the question. The balance history, the third
//! report, is [`super::GetBalanceHistoryUseCase`].

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::{
    ids::{CategoryId, UserId},
    ledger::{
        BudgetFacts, Ledger,
        cards::Month,
        reports::{MonthTotals, ReportPeriodError},
    },
    repositories::{
        card_purchases::CardPurchaseRepository, categories::CategoryRepository,
        credit_cards::CreditCardRepository, opening_balance::OpeningBalanceRepository,
        recurring::RecurringRepository, spending::SpendingRepository,
        transactions::TransactionRepository,
    },
};

use super::ledger_loader::LedgerLoader;

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

internal_error!(ReportError);

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
    facts: LedgerLoader,
    spending: SpendingRepository,
}

impl ReportLoader {
    async fn load(&self, user_id: UserId) -> Result<Ledger, ReportError> {
        // The expenses with their Categories are the same facts the Budgets use.
        let (facts, spending) =
            tokio::try_join!(self.facts.facts(user_id), self.spending.facts(user_id))
                .map_err(ReportError::internal)?;
        Ok(Ledger::new(facts).with_budget_facts(BudgetFacts {
            spending,
            budgets: Vec::new(),
        }))
    }
}

macro_rules! constructor {
    () => {
        pub fn new(
            categories: CategoryRepository,
            cards: CreditCardRepository,
            purchases: CardPurchaseRepository,
            recurring: RecurringRepository,
            transactions: TransactionRepository,
            opening_balance: OpeningBalanceRepository,
            spending: SpendingRepository,
        ) -> Self {
            Self {
                loader: ReportLoader {
                    categories,
                    facts: LedgerLoader::new(
                        transactions,
                        opening_balance,
                        recurring,
                        cards,
                        purchases,
                    ),
                    spending,
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
        let ledger = self.loader.load(user_id).await?;
        let spending = ledger.expenses_by_category(from, to, today)?;

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
            .unwrap_or_else(|| to.plus(1 - DEFAULT_SERIES_MONTHS as i32));
        let ledger = self.loader.load(user_id).await?;
        let months = ledger.income_vs_expenses(from, to, today)?;
        Ok(IncomeVsExpenses {
            from,
            to,
            income_cents: months.iter().map(|month| month.income_cents).sum(),
            expense_cents: months.iter().map(|month| month.expense_cents).sum(),
            months,
        })
    }
}
