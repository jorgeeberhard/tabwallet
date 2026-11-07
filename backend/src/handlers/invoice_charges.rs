//! Charges (Encargos, UC21): interest, penalty and IOF the bank charged on an
//! invoice, recorded by the user. An expense of "Juros e encargos" linked to
//! the invoice; the system never calculates it.

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
    card_purchases::PurchaseCard,
    credit_cards::{InvoiceCycleResponse, InvoiceResponse},
    extract::{ValidatedJson, trimmed_opt},
    transactions::{MAX_AMOUNT_CENTS, MAX_DESCRIPTION_LEN, TransactionCategory},
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::{ChargeId, CreditCardId},
    ledger::cards::Month,
    repositories::{
        card_purchases::CardPurchaseRepository,
        categories::CategoryRepository,
        credit_cards::CreditCardRepository,
        invoice_charges::{ChargeChanges, InvoiceChargeRepository},
    },
    use_cases::{
        ChargeView, CreateChargeUseCase, DeleteChargeUseCase, NewChargeInput, UpdateChargeUseCase,
    },
};

/// The public view of a Charge.
#[derive(Serialize)]
pub struct ChargeResponse {
    id: ChargeId,
    /// Cents, always greater than zero. It is an expense.
    amount_cents: i64,
    /// The day the bank charged it, `YYYY-MM-DD`.
    date: NaiveDate,
    description: String,
    /// Always "Juros e encargos" (the user's default category of Charges).
    category: TransactionCategory,
    credit_card: PurchaseCard,
    /// The invoice the Charge is linked to. Worked out on every read: its
    /// state changes with the clock.
    invoice: InvoiceCycleResponse,
    /// The month the Charge counts in for reports and budgets: the month of
    /// the invoice's due date.
    reference_month: String,
    /// The date is after today (America/Sao_Paulo).
    is_projection: bool,
    created_at: DateTime<Utc>,
}

impl ChargeResponse {
    pub fn new(view: ChargeView, today: NaiveDate) -> Self {
        let charge = view.charge;
        Self {
            id: charge.id,
            amount_cents: charge.amount_cents,
            date: charge.date,
            description: charge.description,
            category: TransactionCategory::new(
                charge.category_id,
                charge.category_name,
                "expense",
                charge.category_active,
            ),
            credit_card: PurchaseCard::new(
                charge.credit_card_id,
                charge.credit_card_name,
                charge.credit_card_active,
            ),
            reference_month: view.invoice.reference_month().to_string(),
            invoice: view.invoice.into(),
            is_projection: charge.date > today,
            created_at: charge.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct ChargeReceiptResponse {
    charge: ChargeResponse,
    /// The invoice after the change, with its new total and remaining amount.
    invoice: InvoiceResponse,
}

#[derive(Deserialize, Validate)]
pub struct CreateRequest {
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: i64,
    date: NaiveDate,
    /// Optional; "Juros e encargos" when absent.
    #[serde(default, deserialize_with = "trimmed_opt")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: Option<String>,
}

/// `POST /credit-cards/{id}/invoices/{closing_month}/charges` (protected):
/// records Charges on an invoice that is not paid (`409 invoice_paid`
/// otherwise). They go to the user's "Juros e encargos" category (`422` with
/// `fields.category_id = category_inactive` if it is deactivated) and join the
/// invoice total.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((id, closing_month)): Path<(String, String)>,
    ValidatedJson(request): ValidatedJson<CreateRequest>,
) -> Result<(StatusCode, Json<ChargeReceiptResponse>), ApiError> {
    let card_id = Uuid::parse_str(&id)
        .map(CreditCardId)
        .map_err(|_| ApiError::CreditCardNotFound)?;
    let month = Month::parse(&closing_month).ok_or(ApiError::InvoiceNotFound)?;
    let today = state.clock.today();
    let receipt = CreateChargeUseCase::new(
        InvoiceChargeRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        card_id,
        month,
        NewChargeInput {
            amount_cents: request.amount_cents,
            date: request.date,
            description: request.description,
        },
        today,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(ChargeReceiptResponse {
            charge: ChargeResponse::new(receipt.charge, today),
            invoice: receipt.invoice.into(),
        }),
    ))
}

/// Every field is optional: only what is present changes. The card, the
/// invoice and the category never change.
#[derive(Deserialize, Validate)]
pub struct UpdateRequest {
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: Option<i64>,
    date: Option<NaiveDate>,
    #[serde(default, deserialize_with = "trimmed_opt")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: Option<String>,
}

/// A malformed id is a Charge that does not exist.
fn parse_id(raw: &str) -> Result<ChargeId, ApiError> {
    Uuid::parse_str(raw)
        .map(ChargeId)
        .map_err(|_| ApiError::ChargeNotFound)
}

/// `PATCH /charges/{id}` (protected): edits amount, date and description
/// until the invoice is paid (`409 invoice_paid` after that).
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateRequest>,
) -> Result<Json<ChargeReceiptResponse>, ApiError> {
    let today = state.clock.today();
    let receipt = UpdateChargeUseCase::new(
        InvoiceChargeRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        parse_id(&id)?,
        ChargeChanges {
            amount_cents: request.amount_cents,
            date: request.date,
            description: request.description,
        },
        today,
    )
    .await?;
    Ok(Json(ChargeReceiptResponse {
        charge: ChargeResponse::new(receipt.charge, today),
        invoice: receipt.invoice.into(),
    }))
}

/// `DELETE /charges/{id}` (protected): removes a Charge until the invoice is
/// paid (`409 invoice_paid` after that).
pub async fn delete(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    DeleteChargeUseCase::new(
        InvoiceChargeRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, parse_id(&id)?, state.clock.today())
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
