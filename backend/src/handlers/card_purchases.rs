use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::{
    credit_cards::InvoiceCycleResponse,
    extract::{ValidatedJson, trimmed, trimmed_opt},
    transactions::{MAX_AMOUNT_CENTS, MAX_DESCRIPTION_LEN, TransactionCategory},
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::{CardPurchaseId, CategoryId, CreditCardId},
    repositories::{
        card_purchases::{CardPurchaseChanges, CardPurchaseRepository, NewCardPurchase},
        categories::CategoryRepository,
        credit_cards::CreditCardRepository,
    },
    use_cases::{
        CardPurchaseView, CreateCardPurchaseUseCase, DeleteCardPurchaseUseCase,
        UpdateCardPurchaseUseCase,
    },
};

/// The card a purchase was made with, as shown with it.
#[derive(Serialize)]
pub struct PurchaseCard {
    id: CreditCardId,
    name: String,
    active: bool,
}

/// The public view of a card purchase.
#[derive(Serialize)]
pub struct CardPurchaseResponse {
    id: CardPurchaseId,
    /// Cents, always greater than zero. It is an expense.
    amount_cents: i64,
    /// The day of the purchase, `YYYY-MM-DD`.
    date: NaiveDate,
    description: String,
    category: TransactionCategory,
    credit_card: PurchaseCard,
    /// The invoice the purchase enters, by the closing rule. Worked out on
    /// every read: changing the date or the card's days reallocates it.
    invoice: InvoiceCycleResponse,
    /// The month the purchase counts in for reports and budgets: the month of
    /// the invoice's due date.
    reference_month: String,
    /// The date is after today (America/Sao_Paulo).
    is_projection: bool,
    created_at: DateTime<Utc>,
}

impl CardPurchaseResponse {
    pub fn new(view: CardPurchaseView, today: NaiveDate) -> Self {
        let purchase = view.purchase;
        Self {
            id: purchase.id,
            amount_cents: purchase.amount_cents,
            date: purchase.date,
            description: purchase.description,
            category: TransactionCategory::new(
                purchase.category_id,
                purchase.category_name,
                "expense",
                purchase.category_active,
            ),
            credit_card: PurchaseCard {
                id: purchase.credit_card_id,
                name: purchase.credit_card_name,
                active: purchase.credit_card_active,
            },
            reference_month: view.invoice.reference_month().to_string(),
            invoice: view.invoice.into(),
            is_projection: purchase.date > today,
            created_at: purchase.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct CardPurchaseEnvelope {
    card_purchase: CardPurchaseResponse,
}

#[derive(Deserialize, Validate)]
pub struct CreateRequest {
    credit_card_id: CreditCardId,
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: i64,
    date: NaiveDate,
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: String,
    category_id: CategoryId,
}

/// `POST /card-purchases` (protected): records a purchase with a credit card.
/// It enters the invoice by the closing rule and does not change the Current
/// Balance.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateRequest>,
) -> Result<(StatusCode, Json<CardPurchaseEnvelope>), ApiError> {
    let today = state.clock.today();
    let view = CreateCardPurchaseUseCase::new(
        CardPurchaseRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        NewCardPurchase {
            credit_card_id: request.credit_card_id,
            amount_cents: request.amount_cents,
            date: request.date,
            description: request.description,
            category_id: request.category_id,
        },
        today,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(CardPurchaseEnvelope {
            card_purchase: CardPurchaseResponse::new(view, today),
        }),
    ))
}

/// Every field is optional: only what is present changes. The card cannot be
/// changed.
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

/// A malformed id is a purchase that does not exist.
fn parse_id(raw: &str) -> Result<CardPurchaseId, ApiError> {
    Uuid::parse_str(raw)
        .map(CardPurchaseId)
        .map_err(|_| ApiError::CardPurchaseNotFound)
}

/// `PATCH /card-purchases/{id}` (protected): edits amount, date, description
/// and category; a new date reallocates the purchase to its invoice. Refused
/// with `409 invoice_paid` when the purchase is in a paid invoice.
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateRequest>,
) -> Result<Json<CardPurchaseEnvelope>, ApiError> {
    let today = state.clock.today();
    let view = UpdateCardPurchaseUseCase::new(
        CardPurchaseRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        parse_id(&id)?,
        CardPurchaseChanges {
            amount_cents: request.amount_cents,
            date: request.date,
            description: request.description,
            category_id: request.category_id,
        },
        today,
    )
    .await?;
    Ok(Json(CardPurchaseEnvelope {
        card_purchase: CardPurchaseResponse::new(view, today),
    }))
}

/// `DELETE /card-purchases/{id}` (protected). Refused with `409 invoice_paid`
/// when the purchase is in a paid invoice.
pub async fn delete(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    DeleteCardPurchaseUseCase::new(
        CardPurchaseRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, parse_id(&id)?, state.clock.today())
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
