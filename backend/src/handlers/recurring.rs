use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::{
    extract::{ValidatedJson, trimmed, trimmed_opt},
    transactions::{MAX_AMOUNT_CENTS, MAX_DESCRIPTION_LEN},
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::{CategoryId, RecurringTransactionId},
    repositories::{
        categories::CategoryRepository, recurring::RecurringRepository,
        transactions::TransactionType,
    },
    use_cases::{
        CreateRecurringTransactionUseCase, EditOccurrenceUseCase, EndRecurringTransactionUseCase,
        ListRecurringTransactionsUseCase, NewRecurring, RecurringChanges, RecurringView,
        SkipOccurrenceUseCase, StatementOccurrence, UpdateRecurringTransactionUseCase,
    },
};

/// A month as `YYYY-MM`; in Rust it is the date of the month's first day.
#[derive(Clone, Copy, Debug)]
pub struct YearMonth(NaiveDate);

impl YearMonth {
    fn parse(text: &str) -> Option<Self> {
        let well_formed = text.len() == 7
            && text.char_indices().all(|(index, c)| {
                if index == 4 {
                    c == '-'
                } else {
                    c.is_ascii_digit()
                }
            });
        if !well_formed {
            return None;
        }
        NaiveDate::parse_from_str(&format!("{text}-01"), "%Y-%m-%d")
            .ok()
            .map(Self)
    }
}

impl<'de> Deserialize<'de> for YearMonth {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| serde::de::Error::custom("expected a month, YYYY-MM"))
    }
}

/// `YYYY-MM` for a date in the month.
fn month_text(date: NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
}

/// The Category of a recurring Transaction, as shown with it.
#[derive(Serialize)]
pub struct RecurringCategory {
    id: CategoryId,
    name: String,
    #[serde(rename = "type")]
    kind: &'static str,
    /// A deactivated category stays on what already uses it.
    active: bool,
}

/// The public view of a recurring Transaction (Gasto fixo or Receita fixa).
#[derive(Serialize)]
pub struct RecurringResponse {
    id: RecurringTransactionId,
    #[serde(rename = "type")]
    kind: &'static str,
    /// Cents, greater than zero: the amount in force this month (the first or
    /// last month's if it has not started or has ended).
    amount_cents: i64,
    day_of_month: u32,
    description: String,
    category: RecurringCategory,
    /// First month with an Occurrence, `YYYY-MM`.
    start_month: String,
    /// Last month with an Occurrence, `YYYY-MM`, once it was ended.
    end_month: Option<String>,
    ended: bool,
    /// The day of the next Occurrence after today, if there is one.
    next_date: Option<NaiveDate>,
    created_at: DateTime<Utc>,
}

impl From<RecurringView> for RecurringResponse {
    fn from(view: RecurringView) -> Self {
        let recurring = view.recurring;
        let kind = recurring.kind.as_str();
        Self {
            id: recurring.id,
            kind,
            amount_cents: view.amount_cents,
            day_of_month: recurring.definition.day_of_month,
            description: recurring.description,
            category: RecurringCategory {
                id: recurring.category_id,
                name: recurring.category_name,
                kind,
                active: recurring.category_active,
            },
            start_month: month_text(recurring.definition.start_month),
            end_month: recurring.definition.end_month.map(month_text),
            ended: recurring.definition.end_month.is_some(),
            next_date: view.next_date,
            created_at: recurring.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct RecurringEnvelope {
    recurring_transaction: RecurringResponse,
}

impl From<RecurringView> for RecurringEnvelope {
    fn from(view: RecurringView) -> Self {
        Self {
            recurring_transaction: view.into(),
        }
    }
}

/// One Occurrence: in the statement (`kind: "occurrence"`) and as the answer
/// to an edit. It is a Projection while its date is after today.
#[derive(Serialize)]
pub struct OccurrenceResponse {
    recurring_id: RecurringTransactionId,
    /// `YYYY-MM`: with `recurring_id`, it identifies the Occurrence.
    month: String,
    #[serde(rename = "type")]
    kind: &'static str,
    /// Cents, always greater than zero; `type` gives the direction.
    amount_cents: i64,
    date: NaiveDate,
    description: String,
    category: RecurringCategory,
    /// The date is after today: it is not in the current balance yet.
    is_projection: bool,
    /// The user edited the amount of this Occurrence only.
    is_edited: bool,
}

impl OccurrenceResponse {
    pub fn new(occurrence: StatementOccurrence, today: NaiveDate) -> Self {
        let kind = occurrence.kind.as_str();
        Self {
            recurring_id: occurrence.recurring_id,
            month: month_text(occurrence.month),
            kind,
            amount_cents: occurrence.amount_cents,
            date: occurrence.date,
            description: occurrence.description,
            category: RecurringCategory {
                id: occurrence.category_id,
                name: occurrence.category_name,
                kind,
                active: occurrence.category_active,
            },
            is_projection: occurrence.date > today,
            is_edited: occurrence.edited,
        }
    }
}

#[derive(Serialize)]
pub struct OccurrenceEnvelope {
    occurrence: OccurrenceResponse,
}

#[derive(Deserialize, Validate)]
pub struct CreateRequest {
    #[serde(rename = "type")]
    kind: TransactionType,
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: i64,
    #[validate(range(min = 1, max = 31, code = "day_of_month_range"))]
    day_of_month: u32,
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: String,
    category_id: CategoryId,
    /// First month with an Occurrence, `YYYY-MM`; defaults to this month and
    /// cannot be an earlier one.
    #[serde(default)]
    start_month: Option<YearMonth>,
}

/// A malformed id is a recurring Transaction that does not exist.
fn parse_id(raw: &str) -> Result<RecurringTransactionId, ApiError> {
    Uuid::parse_str(raw)
        .map(RecurringTransactionId)
        .map_err(|_| ApiError::RecurringTransactionNotFound)
}

/// A malformed month is an Occurrence that does not exist.
fn parse_month(raw: &str) -> Result<NaiveDate, ApiError> {
    YearMonth::parse(raw)
        .map(|month| month.0)
        .ok_or(ApiError::OccurrenceNotFound)
}

/// `POST /recurring-transactions` (protected): registers a Gasto fixo or a
/// Receita fixa. Nothing but the definition is stored.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateRequest>,
) -> Result<(StatusCode, Json<RecurringEnvelope>), ApiError> {
    let view = CreateRecurringTransactionUseCase::new(
        RecurringRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        state.clock.today(),
        NewRecurring {
            kind: request.kind,
            amount_cents: request.amount_cents,
            day_of_month: request.day_of_month,
            description: request.description,
            category_id: request.category_id,
            start_month: request.start_month.map(|month| month.0),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

#[derive(Serialize)]
pub struct ListResponse {
    recurring_transactions: Vec<RecurringResponse>,
}

/// `GET /recurring-transactions` (protected): the user's Gastos fixos and
/// Receitas fixas, ended ones included, oldest first.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<ListResponse>, ApiError> {
    let views = ListRecurringTransactionsUseCase::new(RecurringRepository::new(state.pool.clone()))
        .execute(auth_user.id, state.clock.today())
        .await?;
    Ok(Json(ListResponse {
        recurring_transactions: views.into_iter().map(Into::into).collect(),
    }))
}

/// Every field is optional: only what is present changes. The type and the day
/// of the month cannot be changed.
#[derive(Deserialize, Validate)]
pub struct UpdateRequest {
    /// Applies from this month on; earlier months keep their amount.
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: Option<i64>,
    #[serde(default, deserialize_with = "trimmed_opt")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: Option<String>,
    category_id: Option<CategoryId>,
}

/// `PATCH /recurring-transactions/{id}` (protected): changes the amount (from
/// this month on), the description or the Category.
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateRequest>,
) -> Result<Json<RecurringEnvelope>, ApiError> {
    let view = UpdateRecurringTransactionUseCase::new(
        RecurringRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        parse_id(&id)?,
        state.clock.today(),
        RecurringChanges {
            amount_cents: request.amount_cents,
            description: request.description,
            category_id: request.category_id,
        },
    )
    .await?;
    Ok(Json(view.into()))
}

#[derive(Default, Deserialize)]
pub struct EndRequest {
    /// The last month with an Occurrence, `YYYY-MM`. Defaults to the last one
    /// that has already fallen, which removes every Projection.
    #[serde(default)]
    last_month: Option<YearMonth>,
}

/// `POST /recurring-transactions/{id}/end` (protected): ends it. Future
/// Occurrences disappear; the ones that already fell stay. The body is
/// optional.
pub async fn end(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<RecurringEnvelope>, ApiError> {
    // An empty body means "end now"; anything else must be a valid request.
    let EndRequest { last_month } = if body.is_empty() {
        EndRequest::default()
    } else {
        serde_json::from_slice(&body).map_err(|_| ApiError::InvalidRequest)?
    };
    let view = EndRecurringTransactionUseCase::new(RecurringRepository::new(state.pool.clone()))
        .execute(
            auth_user.id,
            parse_id(&id)?,
            state.clock.today(),
            last_month.map(|month| month.0),
        )
        .await?;
    Ok(Json(view.into()))
}

#[derive(Deserialize, Validate)]
pub struct EditOccurrenceRequest {
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: i64,
}

/// `PATCH /recurring-transactions/{id}/occurrences/{month}` (protected):
/// edits the amount of that single Occurrence; the others keep theirs.
pub async fn edit_occurrence(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((id, month)): Path<(String, String)>,
    ValidatedJson(request): ValidatedJson<EditOccurrenceRequest>,
) -> Result<Json<OccurrenceEnvelope>, ApiError> {
    let view = EditOccurrenceUseCase::new(RecurringRepository::new(state.pool.clone()))
        .execute(
            auth_user.id,
            parse_id(&id)?,
            parse_month(&month)?,
            request.amount_cents,
        )
        .await?;
    Ok(Json(OccurrenceEnvelope {
        occurrence: OccurrenceResponse::new(
            StatementOccurrence::new(&view.recurring, view.occurrence),
            state.clock.today(),
        ),
    }))
}

/// `DELETE /recurring-transactions/{id}/occurrences/{month}` (protected):
/// deletes that single Occurrence; the others stay.
pub async fn delete_occurrence(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((id, month)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    SkipOccurrenceUseCase::new(RecurringRepository::new(state.pool.clone()))
        .execute(auth_user.id, parse_id(&id)?, parse_month(&month)?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
