//! `GET /reports/expenses-by-category` and `GET /reports/income-vs-expenses`:
//! the reports by Reference Month (UC11). The balance history, the third
//! report, is `GET /balance/history`.

use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::CategoryId,
    ledger::{cards::Month, reports::MonthTotals},
    repositories::{
        card_purchases::CardPurchaseRepository, categories::CategoryRepository,
        credit_cards::CreditCardRepository, opening_balance::OpeningBalanceRepository,
        recurring::RecurringRepository, spending::SpendingRepository,
        transactions::TransactionRepository,
    },
    use_cases::{
        CategoryLine, ExpensesByCategory, GetExpensesByCategoryUseCase, GetIncomeVsExpensesUseCase,
        IncomeVsExpenses, ReportPeriod,
    },
};

#[derive(Deserialize)]
pub struct PeriodParams {
    /// First month, inclusive, `YYYY-MM`.
    from: Option<String>,
    /// Last month, inclusive, `YYYY-MM`.
    to: Option<String>,
}

impl PeriodParams {
    fn parse(&self) -> Result<ReportPeriod, ApiError> {
        Ok(ReportPeriod {
            from: parse_month("from", self.from.as_deref())?,
            to: parse_month("to", self.to.as_deref())?,
        })
    }
}

fn parse_month(field: &str, text: Option<&str>) -> Result<Option<Month>, ApiError> {
    let Some(text) = text else {
        return Ok(None);
    };
    Month::parse(text).map(Some).ok_or_else(|| {
        ApiError::Validation(BTreeMap::from([(
            field.to_string(),
            "invalid_month".to_string(),
        )]))
    })
}

fn use_case_repositories(
    state: &AppState,
) -> (
    CategoryRepository,
    CreditCardRepository,
    CardPurchaseRepository,
    RecurringRepository,
    TransactionRepository,
    OpeningBalanceRepository,
    SpendingRepository,
) {
    (
        CategoryRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        RecurringRepository::new(state.pool.clone()),
        TransactionRepository::new(state.pool.clone()),
        OpeningBalanceRepository::new(state.pool.clone()),
        SpendingRepository::new(state.pool.clone()),
    )
}

#[derive(Serialize)]
pub struct CategoryResponse {
    category_id: CategoryId,
    name: String,
    /// A deactivated Category stays in the report of the months it was used.
    active: bool,
    /// Cents.
    total_cents: i64,
    /// Cents: the part of the total that is a Projection (its date is after
    /// today).
    projected_cents: i64,
}

impl From<CategoryLine> for CategoryResponse {
    fn from(line: CategoryLine) -> Self {
        Self {
            category_id: line.category_id,
            name: line.name,
            active: line.active,
            total_cents: line.total_cents,
            projected_cents: line.projected_cents,
        }
    }
}

#[derive(Serialize)]
pub struct ExpensesByCategoryResponse {
    /// The period actually covered, `YYYY-MM`, with the defaults filled in.
    from: String,
    to: String,
    total_cents: i64,
    projected_cents: i64,
    /// The biggest first; Categories with no expense in the period are left out.
    categories: Vec<CategoryResponse>,
}

impl From<ExpensesByCategory> for ExpensesByCategoryResponse {
    fn from(report: ExpensesByCategory) -> Self {
        Self {
            from: report.from.to_string(),
            to: report.to.to_string(),
            total_cents: report.total_cents,
            projected_cents: report.projected_cents,
            categories: report.categories.into_iter().map(Into::into).collect(),
        }
    }
}

/// `GET /reports/expenses-by-category?from=YYYY-MM&to=YYYY-MM` (protected):
/// what each Category spent in the Reference Months of the period (default:
/// the current month). Invoice Payments and the Carried Balance are not
/// expenses; Charges and the Projections of the period are.
pub async fn expenses_by_category(
    State(state): State<AppState>,
    auth_user: AuthUser,
    params: Result<Query<PeriodParams>, QueryRejection>,
) -> Result<Json<ExpensesByCategoryResponse>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::InvalidRequest)?;
    let period = params.parse()?;
    let (categories, cards, purchases, recurring, transactions, opening_balance, spending) =
        use_case_repositories(&state);
    let report = GetExpensesByCategoryUseCase::new(
        categories,
        cards,
        purchases,
        recurring,
        transactions,
        opening_balance,
        spending,
    )
    .execute(auth_user.id, state.clock.today(), period)
    .await?;
    Ok(Json(report.into()))
}

#[derive(Serialize)]
pub struct MonthResponse {
    /// `YYYY-MM`.
    month: String,
    income_cents: i64,
    expense_cents: i64,
    /// Income less expenses; may be negative.
    balance_cents: i64,
    /// The part of the income that is a Projection.
    projected_income_cents: i64,
    /// The part of the expenses that is a Projection.
    projected_expense_cents: i64,
}

impl From<MonthTotals> for MonthResponse {
    fn from(totals: MonthTotals) -> Self {
        Self {
            month: totals.month.to_string(),
            income_cents: totals.income_cents,
            expense_cents: totals.expense_cents,
            balance_cents: totals.income_cents - totals.expense_cents,
            projected_income_cents: totals.projected_income_cents,
            projected_expense_cents: totals.projected_expense_cents,
        }
    }
}

#[derive(Serialize)]
pub struct IncomeVsExpensesResponse {
    /// The period actually covered, `YYYY-MM`, with the defaults filled in.
    from: String,
    to: String,
    /// Totals of the whole period.
    income_cents: i64,
    expense_cents: i64,
    /// One entry per month, oldest first, empty months included.
    months: Vec<MonthResponse>,
}

impl From<IncomeVsExpenses> for IncomeVsExpensesResponse {
    fn from(report: IncomeVsExpenses) -> Self {
        Self {
            from: report.from.to_string(),
            to: report.to.to_string(),
            income_cents: report.income_cents,
            expense_cents: report.expense_cents,
            months: report.months.into_iter().map(Into::into).collect(),
        }
    }
}

/// `GET /reports/income-vs-expenses?from=YYYY-MM&to=YYYY-MM` (protected):
/// income and expenses of each Reference Month of the period (default: the
/// last six months, this one included), by the same rules as the expenses by
/// Category; income includes the Occurrences of recurring incomes.
pub async fn income_vs_expenses(
    State(state): State<AppState>,
    auth_user: AuthUser,
    params: Result<Query<PeriodParams>, QueryRejection>,
) -> Result<Json<IncomeVsExpensesResponse>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::InvalidRequest)?;
    let period = params.parse()?;
    let (categories, cards, purchases, recurring, transactions, opening_balance, spending) =
        use_case_repositories(&state);
    let report = GetIncomeVsExpensesUseCase::new(
        categories,
        cards,
        purchases,
        recurring,
        transactions,
        opening_balance,
        spending,
    )
    .execute(auth_user.id, state.clock.today(), period)
    .await?;
    Ok(Json(report.into()))
}
