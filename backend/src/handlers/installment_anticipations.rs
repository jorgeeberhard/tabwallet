//! Installment Anticipation with a Discount: bringing Installments of
//! invoices after the open one into the open invoice of the same card.

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::{
    credit_cards::{InvoiceCycleResponse, InvoiceResponse},
    extract::ValidatedJson,
    installment_purchases::InvoiceInstallmentResponse,
    transactions::MAX_AMOUNT_CENTS,
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::{CreditCardId, InstallmentId},
    repositories::{
        card_purchases::CardPurchaseRepository, credit_cards::CreditCardRepository,
        installment_purchases::InstallmentPurchaseRepository,
    },
    use_cases::{AnticipateInstallmentsUseCase, ListAnticipableInstallmentsUseCase},
};

/// A malformed id is a card that does not exist.
fn parse_card_id(raw: &str) -> Result<CreditCardId, ApiError> {
    Uuid::parse_str(raw)
        .map(CreditCardId)
        .map_err(|_| ApiError::CreditCardNotFound)
}

#[derive(Serialize)]
pub struct AnticipableResponse {
    /// The invoice that is open today: where anticipated Installments go.
    open_invoice: InvoiceCycleResponse,
    /// The Installments in invoices after the open one, earliest invoice
    /// first.
    installments: Vec<InvoiceInstallmentResponse>,
}

/// `GET /credit-cards/{id}/anticipable-installments` (protected): the
/// Installments of the card that can be anticipated now, with the open
/// invoice that would receive them.
pub async fn list_anticipable(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<AnticipableResponse>, ApiError> {
    let found = ListAnticipableInstallmentsUseCase::new(
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        InstallmentPurchaseRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, parse_card_id(&id)?, state.clock.today())
    .await?;
    Ok(Json(AnticipableResponse {
        open_invoice: found.open_invoice.into(),
        installments: found.installments.into_iter().map(Into::into).collect(),
    }))
}

#[derive(Deserialize, Validate)]
pub struct AnticipateRequest {
    /// The Installments to bring to the open invoice; all of this card.
    #[serde(default)]
    #[validate(length(min = 1, code = "installments_required"))]
    installment_ids: Vec<InstallmentId>,
    /// Cents of Discount, greater than zero; leave out (or `null`) for none.
    /// Split between the Installments in proportion to their amounts and never
    /// enough to leave one without a cent.
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "discount_range"))]
    discount_cents: Option<i64>,
    /// The operation cannot be undone, so it only happens with `true` here.
    #[serde(default)]
    confirm: bool,
}

#[derive(Serialize)]
pub struct AnticipationResponse {
    credit_card_id: CreditCardId,
    /// Cents the Installments added up to before the Discount.
    original_total_cents: i64,
    /// Cents of Discount actually split between them.
    discount_cents: i64,
    /// Cents the Installments add up to in the open invoice.
    total_cents: i64,
    /// The open invoice after receiving them.
    invoice: InvoiceResponse,
    /// The anticipated Installments as they are now, earliest origin first.
    installments: Vec<InvoiceInstallmentResponse>,
}

#[derive(Serialize)]
pub struct AnticipationEnvelope {
    anticipation: AnticipationResponse,
}

/// `POST /credit-cards/{id}/installment-anticipations` (protected): brings the
/// chosen Installments to the card's open invoice. They count in that
/// invoice's Reference Month from then on. An optional Discount is split in
/// proportion between them, reducing each (and so the expense in the Category
/// of its purchase). **Irreversible**: there is no route that undoes it, and
/// it only runs with `"confirm": true` (`422` with
/// `fields.confirm = confirmation_required` otherwise).
///
/// Other answers: `422` `fields.installment_ids` = `installments_required` or
/// `installment_not_found` (not of this card or user) and `fields.discount_cents`
/// = `discount_range` or `discount_exceeds_total`; `409
/// installments_not_anticipable` when a chosen Installment is not in an invoice
/// after the open one; `404 credit_card_not_found`.
pub async fn anticipate(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<AnticipateRequest>,
) -> Result<Json<AnticipationEnvelope>, ApiError> {
    let card_id = parse_card_id(&id)?;
    let receipt = AnticipateInstallmentsUseCase::new(
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        InstallmentPurchaseRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        card_id,
        request.installment_ids,
        request.discount_cents.unwrap_or(0),
        request.confirm,
        state.clock.today(),
    )
    .await?;
    Ok(Json(AnticipationEnvelope {
        anticipation: AnticipationResponse {
            credit_card_id: card_id,
            original_total_cents: receipt.original_total_cents,
            discount_cents: receipt.discount_cents,
            total_cents: receipt.total_cents,
            invoice: receipt.invoice.into(),
            installments: receipt.installments.into_iter().map(Into::into).collect(),
        },
    }))
}
