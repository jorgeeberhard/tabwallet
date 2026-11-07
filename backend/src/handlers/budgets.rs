use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::{extract::ValidatedJson, transactions::MAX_AMOUNT_CENTS};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::CategoryId,
    ledger::cards::Month,
    repositories::{
        budgets::BudgetRepository, card_purchases::CardPurchaseRepository,
        categories::CategoryRepository, credit_cards::CreditCardRepository,
        recurring::RecurringRepository, spending::SpendingRepository,
    },
    use_cases::{
        BudgetCategory, BudgetReport, BudgetStatusLine, BudgetView, GetBudgetStatusUseCase,
        ListBudgetsUseCase, RemoveBudgetUseCase, SetBudgetUseCase,
    },
};

/// The Category of a Budget. Always an expense one.
#[derive(Serialize)]
pub struct BudgetCategoryResponse {
    id: CategoryId,
    name: String,
    #[serde(rename = "type")]
    kind: &'static str,
    /// A deactivated Category keeps its Budget, which then produces no Alert.
    active: bool,
}

impl From<BudgetCategory> for BudgetCategoryResponse {
    fn from(category: BudgetCategory) -> Self {
        Self {
            id: category.id,
            name: category.name,
            kind: "expense",
            active: category.active,
        }
    }
}

/// The public view of a Budget: the limit in force and the month (`YYYY-MM`)
/// it started in.
#[derive(Serialize)]
pub struct BudgetResponse {
    category: BudgetCategoryResponse,
    /// Cents, greater than zero. Applies every month from `effective_from`
    /// until changed or removed.
    limit_cents: i64,
    effective_from: String,
}

impl From<BudgetView> for BudgetResponse {
    fn from(view: BudgetView) -> Self {
        Self {
            category: view.category.into(),
            limit_cents: view.limit_cents,
            effective_from: view.effective_from.to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct BudgetEnvelope {
    budget: BudgetResponse,
}

#[derive(Serialize)]
pub struct BudgetsEnvelope {
    budgets: Vec<BudgetResponse>,
}

#[derive(Deserialize, Validate)]
pub struct SetBudgetRequest {
    /// Cents.
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    limit_cents: i64,
}

fn category_not_found() -> ApiError {
    ApiError::Validation(
        [("category_id".to_string(), "category_not_found".to_string())]
            .into_iter()
            .collect(),
    )
}

/// `PUT /budgets/{category_id}` (protected) `{limit_cents}`: defines or
/// changes the Budget of an active expense Category. The limit applies from
/// the current month on; earlier months keep theirs.
pub async fn set(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(category_id): Path<String>,
    ValidatedJson(request): ValidatedJson<SetBudgetRequest>,
) -> Result<Json<BudgetEnvelope>, ApiError> {
    // A malformed id is a Category that does not exist.
    let category_id = Uuid::parse_str(&category_id)
        .map(CategoryId)
        .map_err(|_| category_not_found())?;
    let view = SetBudgetUseCase::new(
        BudgetRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        category_id,
        request.limit_cents,
        state.clock.today(),
    )
    .await?;
    Ok(Json(BudgetEnvelope {
        budget: view.into(),
    }))
}

/// `GET /budgets` (protected): the Budgets in force this month, by Category
/// name, including those of deactivated Categories.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<BudgetsEnvelope>, ApiError> {
    let views = ListBudgetsUseCase::new(BudgetRepository::new(state.pool.clone()))
        .execute(auth_user.id, state.clock.today())
        .await?;
    Ok(Json(BudgetsEnvelope {
        budgets: views.into_iter().map(Into::into).collect(),
    }))
}

/// `DELETE /budgets/{category_id}` (protected): removes the Budget from the
/// current month on; earlier months keep theirs. `404 budget_not_found` if
/// the Category has none.
pub async fn remove(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(category_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let category_id = Uuid::parse_str(&category_id)
        .map(CategoryId)
        .map_err(|_| ApiError::BudgetNotFound)?;
    RemoveBudgetUseCase::new(BudgetRepository::new(state.pool.clone()))
        .execute(auth_user.id, category_id, state.clock.today())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct StatusParams {
    /// The Reference Month, `YYYY-MM`; the current month by default.
    month: Option<String>,
}

/// One Category against its Budget.
#[derive(Serialize)]
pub struct BudgetStatusResponse {
    category: BudgetCategoryResponse,
    limit_cents: i64,
    /// The sum of the Category's expenses in the month, Projections included.
    spent_cents: i64,
    /// The limit less the spending; negative once the limit is exceeded.
    remaining_cents: i64,
    /// The spending as a whole percentage of the limit, rounded down.
    percent_used: i64,
    /// The Budget Alert: the spending is above the limit and the Category is
    /// active. A deactivated Category never has one.
    exceeded: bool,
}

impl From<BudgetStatusLine> for BudgetStatusResponse {
    fn from(line: BudgetStatusLine) -> Self {
        Self {
            category: line.category.into(),
            limit_cents: line.status.limit_cents,
            spent_cents: line.status.spent_cents,
            remaining_cents: line.status.limit_cents - line.status.spent_cents,
            percent_used: line.status.percent_used,
            exceeded: line.status.exceeded,
        }
    }
}

#[derive(Serialize)]
pub struct BudgetStatusEnvelope {
    month: String,
    /// How many Categories have a Budget Alert in the month.
    exceeded_count: usize,
    budgets: Vec<BudgetStatusResponse>,
}

impl From<BudgetReport> for BudgetStatusEnvelope {
    fn from(report: BudgetReport) -> Self {
        Self {
            month: report.month.to_string(),
            exceeded_count: report.exceeded_count(),
            budgets: report.lines.into_iter().map(Into::into).collect(),
        }
    }
}

/// `GET /budgets/status?month=YYYY-MM` (protected): the spending of each
/// Category that has a Budget in the Reference Month against its limit, with
/// the Budget Alert. Calculated on every read.
pub async fn status(
    State(state): State<AppState>,
    auth_user: AuthUser,
    params: Result<Query<StatusParams>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<BudgetStatusEnvelope>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::InvalidRequest)?;
    let month = match params.month {
        Some(text) => Month::parse(&text).ok_or(ApiError::InvalidRequest)?,
        None => Month::of(state.clock.today()),
    };
    let report = GetBudgetStatusUseCase::new(
        BudgetRepository::new(state.pool.clone()),
        SpendingRepository::new(state.pool.clone()),
        RecurringRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, month)
    .await?;
    Ok(Json(report.into()))
}
