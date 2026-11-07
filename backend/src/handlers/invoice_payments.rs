//! Paying an invoice (UC17) and how payments are shown: an Early Payment made
//! by the user, the automatic payment of the due date, and the statement item
//! that carries both.

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
    credit_cards::{InvoiceCycleResponse, InvoiceResponse},
    extract::ValidatedJson,
    transactions::MAX_AMOUNT_CENTS,
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::{CreditCardId, InvoicePaymentId},
    ledger::cards::{InvoicePayment, Month, PaymentOrigin},
    repositories::{
        card_purchases::CardPurchaseRepository, credit_cards::CreditCardRepository,
        invoice_payments::InvoicePaymentRepository,
    },
    use_cases::{PayInvoiceUseCase, StatementInvoicePayment},
};

/// One payment of an invoice: an Early Payment the user made, or the automatic
/// one of the due date. Neither is an expense.
#[derive(Serialize)]
pub struct InvoicePaymentResponse {
    /// Only an Early Payment is stored, so only it has an id.
    id: Option<InvoicePaymentId>,
    /// `early` (made by the user before the due date) or `automatic` (debited
    /// on the due date, derived on every read).
    origin: &'static str,
    /// Cents, greater than zero.
    amount_cents: i64,
    /// The day the money leaves (or left) the Current Balance.
    date: NaiveDate,
    /// The date is after today: an automatic payment not yet due, which the
    /// Projected Balance already subtracts.
    is_projection: bool,
}

impl InvoicePaymentResponse {
    pub fn new(payment: &InvoicePayment, today: NaiveDate) -> Self {
        let (id, origin) = match payment.origin {
            PaymentOrigin::Early(id) => (Some(id), "early"),
            PaymentOrigin::Automatic => (None, "automatic"),
        };
        Self {
            id,
            origin,
            amount_cents: payment.amount_cents,
            date: payment.date,
            is_projection: payment.date > today,
        }
    }
}

/// The card an invoice payment was made for, as shown with it.
#[derive(Serialize)]
pub struct PaymentCard {
    id: CreditCardId,
    name: String,
    active: bool,
}

/// An Invoice Payment as an entry of the statement (`kind: "invoice_payment"`).
/// It is not a Transaction and has no Category: it is the settling of an
/// invoice, shown with the card and the invoice it pays.
#[derive(Serialize)]
pub struct StatementInvoicePaymentResponse {
    #[serde(flatten)]
    payment: InvoicePaymentResponse,
    credit_card: PaymentCard,
    /// The invoice it pays.
    invoice: InvoiceCycleResponse,
    /// When it was recorded; `null` for the automatic payment, which is
    /// derived.
    created_at: Option<DateTime<Utc>>,
}

impl StatementInvoicePaymentResponse {
    pub fn new(item: StatementInvoicePayment, today: NaiveDate) -> Self {
        Self {
            payment: InvoicePaymentResponse::new(&item.payment, today),
            credit_card: PaymentCard {
                id: item.payment.card_id,
                name: item.credit_card_name,
                active: item.credit_card_active,
            },
            invoice: item.invoice.into(),
            created_at: item.created_at,
        }
    }
}

#[derive(Deserialize, Validate)]
pub struct PayRequest {
    /// At most what the invoice still has to pay (checked against the
    /// invoice, not here).
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    amount_cents: i64,
}

#[derive(Serialize)]
pub struct PayResponse {
    payment: InvoicePaymentResponse,
    /// The invoice after the payment.
    invoice: InvoiceResponse,
}

/// `POST /credit-cards/{id}/invoices/{closing_month}/payments` (protected):
/// pays an Open or Closed invoice early, in full or in part. The payment is
/// dated today. `409 invoice_paid` for a Paid invoice; `422` with
/// `fields.amount_cents = amount_exceeds_remaining` for more than it owes.
pub async fn pay(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((id, closing_month)): Path<(String, String)>,
    ValidatedJson(request): ValidatedJson<PayRequest>,
) -> Result<(StatusCode, Json<PayResponse>), ApiError> {
    let card_id = Uuid::parse_str(&id)
        .map(CreditCardId)
        .map_err(|_| ApiError::CreditCardNotFound)?;
    let month = Month::parse(&closing_month).ok_or(ApiError::InvoiceNotFound)?;
    let today = state.clock.today();
    let receipt = PayInvoiceUseCase::new(
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        InvoicePaymentRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, card_id, month, request.amount_cents, today)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(PayResponse {
            payment: InvoicePaymentResponse {
                id: Some(receipt.payment.id),
                origin: "early",
                amount_cents: receipt.payment.amount_cents,
                date: receipt.payment.date,
                is_projection: false,
            },
            invoice: receipt.invoice.into(),
        }),
    ))
}
