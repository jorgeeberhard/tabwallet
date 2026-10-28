use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::extract::{ValidatedJson, trimmed};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::{CategoryId, TransactionId},
    repositories::categories::CategoryRepository,
    repositories::transactions::{
        NewTransaction, Transaction, TransactionRepository, TransactionType,
    },
    use_cases::{CreateTransactionUseCase, ListTransactionsUseCase, Statement, StatementItem},
};

/// The largest amount accepted, in cents (R$ 1 billion): far above any
/// personal transaction and far below what `BIGINT` can hold.
pub const MAX_AMOUNT_CENTS: i64 = 100_000_000_000;
pub const MAX_DESCRIPTION_LEN: u64 = 200;
pub const DEFAULT_LIMIT: i64 = 50;

/// The Category of a transaction, as shown with it.
#[derive(Serialize)]
pub struct TransactionCategory {
    id: CategoryId,
    name: String,
    #[serde(rename = "type")]
    kind: &'static str,
    /// A deactivated category stays on the transactions that already use it.
    active: bool,
}

/// The public view of a Transaction.
#[derive(Serialize)]
pub struct TransactionResponse {
    id: TransactionId,
    #[serde(rename = "type")]
    kind: &'static str,
    /// Cents, always greater than zero; `type` gives the direction.
    amount_cents: i64,
    /// A calendar day, `YYYY-MM-DD`.
    date: NaiveDate,
    description: String,
    category: TransactionCategory,
    /// The date is after today (America/Sao_Paulo): it is not in the current
    /// balance yet. Computed on every read, never stored.
    is_projection: bool,
    created_at: DateTime<Utc>,
}

impl TransactionResponse {
    fn new(transaction: Transaction, today: NaiveDate) -> Self {
        Self {
            id: transaction.id,
            kind: transaction.kind.as_str(),
            amount_cents: transaction.amount_cents,
            date: transaction.date,
            description: transaction.description,
            category: TransactionCategory {
                id: transaction.category_id,
                name: transaction.category_name,
                kind: transaction.kind.as_str(),
                active: transaction.category_active,
            },
            is_projection: transaction.date > today,
            created_at: transaction.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct TransactionEnvelope {
    transaction: TransactionResponse,
}

#[derive(Deserialize, Validate)]
pub struct CreateRequest {
    #[serde(rename = "type")]
    kind: TransactionType,
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: i64,
    date: NaiveDate,
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: String,
    category_id: CategoryId,
}

/// `POST /transactions` (protected): records an income or an expense outside
/// the credit card.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateRequest>,
) -> Result<(StatusCode, Json<TransactionEnvelope>), ApiError> {
    let transaction = CreateTransactionUseCase::new(
        TransactionRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        NewTransaction {
            kind: request.kind,
            amount_cents: request.amount_cents,
            date: request.date,
            description: request.description,
            category_id: request.category_id,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(TransactionEnvelope {
            transaction: TransactionResponse::new(transaction, state.clock.today()),
        }),
    ))
}

/// One entry of the statement; `kind` says what the rest of the object is, so
/// card purchases and invoice payments can join later with their own shapes.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatementItemResponse {
    Transaction(TransactionResponse),
}

#[derive(Serialize)]
pub struct StatementResponse {
    items: Vec<StatementItemResponse>,
    /// How many items match the filters, across all pages.
    total: i64,
    limit: i64,
    offset: i64,
}

/// `GET /transactions` (protected): the user's statement, newest first.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<StatementResponse>, ApiError> {
    let (limit, offset) = (DEFAULT_LIMIT, 0);
    let Statement { items, total } =
        ListTransactionsUseCase::new(TransactionRepository::new(state.pool.clone()))
            .execute(auth_user.id, limit, offset)
            .await?;
    let today = state.clock.today();
    Ok(Json(StatementResponse {
        items: items
            .into_iter()
            .map(|item| match item {
                StatementItem::Transaction(transaction) => {
                    StatementItemResponse::Transaction(TransactionResponse::new(transaction, today))
                }
            })
            .collect(),
        total,
        limit,
        offset,
    }))
}
