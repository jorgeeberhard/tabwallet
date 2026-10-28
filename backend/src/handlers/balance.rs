use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{extract::ValidatedJson, transactions::MAX_AMOUNT_CENTS};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    repositories::{
        opening_balance::OpeningBalanceRepository, transactions::TransactionRepository,
    },
    use_cases::{BalanceReport, GetBalanceUseCase, SetOpeningBalanceUseCase},
};

/// The public view of the user's balance.
#[derive(Serialize)]
pub struct BalanceResponse {
    /// Cents, as of today (America/Sao_Paulo); may be negative. Projections
    /// (dates after today) are not included.
    current_balance_cents: i64,
    /// Whether the user informed an Opening Balance (it is informed once).
    opening_balance_set: bool,
    /// The Opening Balance in cents (zero and negative are valid), or `null`.
    opening_balance_cents: Option<i64>,
}

impl From<BalanceReport> for BalanceResponse {
    fn from(report: BalanceReport) -> Self {
        Self {
            current_balance_cents: report.current_balance_cents,
            opening_balance_set: report.opening_balance_cents.is_some(),
            opening_balance_cents: report.opening_balance_cents,
        }
    }
}

async fn current(state: &AppState, auth_user: &AuthUser) -> Result<BalanceResponse, ApiError> {
    let report = GetBalanceUseCase::new(
        TransactionRepository::new(state.pool.clone()),
        OpeningBalanceRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, state.clock.today())
    .await?;
    Ok(report.into())
}

/// `GET /balance` (protected): the Current Balance and the Opening Balance.
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<BalanceResponse>, ApiError> {
    Ok(Json(current(&state, &auth_user).await?))
}

#[derive(Deserialize, Validate)]
pub struct SetOpeningBalanceRequest {
    /// Cents; zero and negative values are allowed.
    #[validate(range(min = -MAX_AMOUNT_CENTS, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: i64,
}

/// `POST /balance/opening` (protected): informs the Opening Balance. It can
/// be done once; a second attempt is `409 opening_balance_already_set`.
/// Answers with the same body as `GET /balance`.
pub async fn set_opening(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<SetOpeningBalanceRequest>,
) -> Result<(StatusCode, Json<BalanceResponse>), ApiError> {
    SetOpeningBalanceUseCase::new(OpeningBalanceRepository::new(state.pool.clone()))
        .execute(auth_user.id, request.amount_cents)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(current(&state, &auth_user).await?),
    ))
}
