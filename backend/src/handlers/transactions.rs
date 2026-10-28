use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::{
    card_purchases::CardPurchaseResponse,
    extract::{ValidatedJson, trimmed, trimmed_opt},
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::{CategoryId, CreditCardId, TransactionId},
    repositories::card_purchases::CardPurchaseRepository,
    repositories::categories::CategoryRepository,
    repositories::credit_cards::CreditCardRepository,
    repositories::statement::StatementRepository,
    repositories::transactions::{
        NewTransaction, Transaction, TransactionChanges, TransactionFilter, TransactionRepository,
        TransactionType,
    },
    use_cases::{
        CreateTransactionUseCase, DeleteTransactionUseCase, ListTransactionsUseCase, Statement,
        StatementItem, UpdateTransactionUseCase,
    },
};

/// The largest amount accepted, in cents (R$ 1 billion): far above any
/// personal transaction and far below what `BIGINT` can hold.
pub const MAX_AMOUNT_CENTS: i64 = 100_000_000_000;
pub const MAX_DESCRIPTION_LEN: u64 = 200;
pub const DEFAULT_LIMIT: i64 = 50;
pub const MAX_LIMIT: i64 = 200;

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

impl TransactionCategory {
    pub fn new(id: CategoryId, name: String, kind: &'static str, active: bool) -> Self {
        Self {
            id,
            name,
            kind,
            active,
        }
    }
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

/// One entry of the statement; `kind` says what the rest of the object is
/// (`transaction` or `card_purchase`), so invoice payments can join later with
/// their own shape.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatementItemResponse {
    Transaction(TransactionResponse),
    CardPurchase(CardPurchaseResponse),
}

#[derive(Serialize)]
pub struct StatementResponse {
    items: Vec<StatementItemResponse>,
    /// How many items match the filters, across all pages.
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Deserialize)]
pub struct ListParams {
    /// First day, inclusive.
    from: Option<NaiveDate>,
    /// Last day, inclusive.
    to: Option<NaiveDate>,
    category_id: Option<CategoryId>,
    #[serde(rename = "type")]
    kind: Option<TransactionType>,
    /// Only purchases made with this credit card.
    credit_card_id: Option<CreditCardId>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl ListParams {
    /// The filter and the page, or the fields that are out of range.
    fn parse(self) -> Result<(TransactionFilter, i64, i64), ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_LIMIT);
        let offset = self.offset.unwrap_or(0);
        let mut fields = BTreeMap::new();
        if !(1..=MAX_LIMIT).contains(&limit) {
            fields.insert("limit".to_string(), "limit_range".to_string());
        }
        if offset < 0 {
            fields.insert("offset".to_string(), "offset_range".to_string());
        }
        if let (Some(from), Some(to)) = (self.from, self.to)
            && from > to
        {
            fields.insert("to".to_string(), "period_order".to_string());
        }
        if !fields.is_empty() {
            return Err(ApiError::Validation(fields));
        }
        let filter = TransactionFilter {
            from: self.from,
            to: self.to,
            category_id: self.category_id,
            kind: self.kind,
            credit_card_id: self.credit_card_id,
        };
        Ok((filter, limit, offset))
    }
}

/// `GET /transactions` (protected): the user's statement, newest first.
/// Optional query: `from` and `to` (days, inclusive), `category_id`,
/// `type=income|expense`, `credit_card_id` (only that card's purchases),
/// `limit` (1 to 200, default 50) and `offset`. Card purchases appear as
/// items with `kind: "card_purchase"`, showing the card and the invoice.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
    params: Result<Query<ListParams>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<StatementResponse>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::InvalidRequest)?;
    let (filter, limit, offset) = params.parse()?;
    let today = state.clock.today();
    let Statement { items, total } = ListTransactionsUseCase::new(
        StatementRepository::new(state.pool.clone()),
        TransactionRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, filter, limit, offset, today)
    .await?;
    Ok(Json(StatementResponse {
        items: items
            .into_iter()
            .map(|item| match item {
                StatementItem::Transaction(transaction) => {
                    StatementItemResponse::Transaction(TransactionResponse::new(transaction, today))
                }
                StatementItem::CardPurchase(view) => {
                    StatementItemResponse::CardPurchase(CardPurchaseResponse::new(view, today))
                }
            })
            .collect(),
        total,
        limit,
        offset,
    }))
}

/// Every field is optional: only what is present changes. The type cannot be
/// changed, so a `type` in the body is ignored.
#[derive(Deserialize, Validate)]
pub struct UpdateRequest {
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: Option<i64>,
    date: Option<NaiveDate>,
    #[serde(default, deserialize_with = "trimmed_opt")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: Option<String>,
    category_id: Option<CategoryId>,
}

/// A malformed id is a transaction that does not exist.
fn parse_id(raw: &str) -> Result<TransactionId, ApiError> {
    Uuid::parse_str(raw)
        .map(TransactionId)
        .map_err(|_| ApiError::TransactionNotFound)
}

/// `PATCH /transactions/{id}` (protected): edits amount, date, description
/// and category.
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateRequest>,
) -> Result<Json<TransactionEnvelope>, ApiError> {
    let transaction = UpdateTransactionUseCase::new(
        TransactionRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        parse_id(&id)?,
        TransactionChanges {
            amount_cents: request.amount_cents,
            date: request.date,
            description: request.description,
            category_id: request.category_id,
        },
    )
    .await?;
    Ok(Json(TransactionEnvelope {
        transaction: TransactionResponse::new(transaction, state.clock.today()),
    }))
}

/// `DELETE /transactions/{id}` (protected).
pub async fn delete(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    DeleteTransactionUseCase::new(TransactionRepository::new(state.pool.clone()))
        .execute(auth_user.id, parse_id(&id)?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
