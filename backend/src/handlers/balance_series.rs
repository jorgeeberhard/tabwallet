//! `GET /balance/history` and `GET /balance/projection`: the balance over
//! time (UC10).

use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ledger::{BalancePoint, Granularity},
    repositories::{
        opening_balance::OpeningBalanceRepository, recurring::RecurringRepository,
        transactions::TransactionRepository,
    },
    use_cases::{
        BalanceHistory, GetBalanceHistoryUseCase, GetProjectedBalanceUseCase, HistoryQuery,
        ProjectedBalance,
    },
};

/// `day` (the default) or `month`.
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GranularityParam {
    #[default]
    Day,
    Month,
}

impl From<GranularityParam> for Granularity {
    fn from(param: GranularityParam) -> Self {
        match param {
            GranularityParam::Day => Granularity::Day,
            GranularityParam::Month => Granularity::Month,
        }
    }
}

fn granularity_name(granularity: Granularity) -> &'static str {
    match granularity {
        Granularity::Day => "day",
        Granularity::Month => "month",
    }
}

#[derive(Serialize)]
pub struct PointResponse {
    /// A calendar day, `YYYY-MM-DD`: the day the balance is taken at the end
    /// of. For monthly series, the last day of the month (or of the period).
    date: NaiveDate,
    /// Cents; may be negative.
    balance_cents: i64,
}

impl From<BalancePoint> for PointResponse {
    fn from(point: BalancePoint) -> Self {
        Self {
            date: point.date,
            balance_cents: point.balance_cents,
        }
    }
}

#[derive(Deserialize)]
pub struct HistoryParams {
    /// First day, inclusive.
    from: Option<NaiveDate>,
    /// Last day, inclusive; not after today.
    to: Option<NaiveDate>,
    granularity: Option<GranularityParam>,
}

#[derive(Serialize)]
pub struct HistoryResponse {
    granularity: &'static str,
    /// The period actually covered, with the defaults filled in.
    from: NaiveDate,
    to: NaiveDate,
    points: Vec<PointResponse>,
}

impl From<BalanceHistory> for HistoryResponse {
    fn from(history: BalanceHistory) -> Self {
        Self {
            granularity: granularity_name(history.granularity),
            from: history.from,
            to: history.to,
            points: history.points.into_iter().map(Into::into).collect(),
        }
    }
}

/// `GET /balance/history?from=&to=&granularity=day|month` (protected): the
/// Current Balance at the end of each day or month of the period.
pub async fn history(
    State(state): State<AppState>,
    auth_user: AuthUser,
    params: Result<Query<HistoryParams>, QueryRejection>,
) -> Result<Json<HistoryResponse>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::InvalidRequest)?;
    let history = GetBalanceHistoryUseCase::new(
        TransactionRepository::new(state.pool.clone()),
        OpeningBalanceRepository::new(state.pool.clone()),
        RecurringRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        state.clock.today(),
        HistoryQuery {
            granularity: params.granularity.unwrap_or_default().into(),
            from: params.from,
            to: params.to,
        },
    )
    .await?;
    Ok(Json(history.into()))
}

#[derive(Deserialize)]
pub struct ProjectionParams {
    /// Last day, inclusive; from today up to the end of the 12th month ahead.
    to: Option<NaiveDate>,
    granularity: Option<GranularityParam>,
}

#[derive(Serialize)]
pub struct ProjectionResponse {
    granularity: &'static str,
    /// Today (America/Sao_Paulo): where the history ends and the projection
    /// starts.
    today: NaiveDate,
    /// The last day covered, with the default (the horizon) filled in.
    to: NaiveDate,
    /// The Current Balance today, in cents: the starting point.
    current_balance_cents: i64,
    points: Vec<PointResponse>,
}

impl From<ProjectedBalance> for ProjectionResponse {
    fn from(projection: ProjectedBalance) -> Self {
        Self {
            granularity: granularity_name(projection.granularity),
            today: projection.today,
            to: projection.to,
            current_balance_cents: projection.current_balance_cents,
            points: projection.points.into_iter().map(Into::into).collect(),
        }
    }
}

/// `GET /balance/projection?to=&granularity=day|month` (protected): the
/// Projected Balance at the end of each day or month from today to `to`.
pub async fn projection(
    State(state): State<AppState>,
    auth_user: AuthUser,
    params: Result<Query<ProjectionParams>, QueryRejection>,
) -> Result<Json<ProjectionResponse>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::InvalidRequest)?;
    let projection = GetProjectedBalanceUseCase::new(
        TransactionRepository::new(state.pool.clone()),
        OpeningBalanceRepository::new(state.pool.clone()),
        RecurringRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        state.clock.today(),
        params.granularity.unwrap_or_default().into(),
        params.to,
    )
    .await?;
    Ok(Json(projection.into()))
}
