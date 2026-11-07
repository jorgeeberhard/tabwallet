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
    card_purchases::CardPurchaseResponse,
    extract::{ValidatedJson, trimmed, trimmed_opt},
    installment_purchases::InvoiceInstallmentResponse,
    invoice_charges::ChargeResponse,
    invoice_payments::InvoicePaymentResponse,
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::CreditCardId,
    ledger::cards::{CorrectionState, Invoice, InvoiceCycle, InvoiceState, Month},
    repositories::{
        card_purchases::CardPurchaseRepository, credit_cards::CreditCardRepository,
        installment_purchases::InstallmentPurchaseRepository,
        invoice_charges::InvoiceChargeRepository,
    },
    use_cases::{
        CreateCreditCardUseCase, CreditCardChanges, CreditCardView, DeactivateCreditCardUseCase,
        GetInvoiceUseCase, ListCreditCardsUseCase, ListInvoicesUseCase,
        ReactivateCreditCardUseCase, UpdateCreditCardUseCase,
    },
};

pub const MAX_NAME_LEN: u64 = 60;

/// The public view of a credit card.
#[derive(Serialize)]
pub struct CreditCardResponse {
    id: CreditCardId,
    name: String,
    /// 1 to 31; in shorter months the last day applies. The days the card has
    /// from now on (invoices already closed keep the days they had).
    closing_day: u32,
    /// 1 to 31, with the same rule.
    due_day: u32,
    /// A deactivated card takes no new purchases but keeps its history.
    active: bool,
    created_at: DateTime<Utc>,
}

impl From<CreditCardView> for CreditCardResponse {
    fn from(view: CreditCardView) -> Self {
        Self {
            id: view.card.id,
            name: view.card.name,
            closing_day: view.closing_day,
            due_day: view.due_day,
            active: view.card.active,
            created_at: view.card.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct CreditCardEnvelope {
    credit_card: CreditCardResponse,
}

fn envelope(view: CreditCardView) -> Json<CreditCardEnvelope> {
    Json(CreditCardEnvelope {
        credit_card: view.into(),
    })
}

/// A malformed id is a card that does not exist.
fn parse_id(raw: &str) -> Result<CreditCardId, ApiError> {
    Uuid::parse_str(raw)
        .map(CreditCardId)
        .map_err(|_| ApiError::CreditCardNotFound)
}

#[derive(Deserialize, Validate)]
pub struct CreateRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, max = MAX_NAME_LEN, code = "name_length"))]
    name: String,
    #[validate(range(min = 1, max = 31, code = "day_range"))]
    closing_day: u32,
    #[validate(range(min = 1, max = 31, code = "day_range"))]
    due_day: u32,
}

/// `POST /credit-cards` (protected): registers a card.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateRequest>,
) -> Result<(StatusCode, Json<CreditCardEnvelope>), ApiError> {
    let view = CreateCreditCardUseCase::new(CreditCardRepository::new(state.pool.clone()))
        .execute(
            auth_user.id,
            &request.name,
            request.closing_day,
            request.due_day,
        )
        .await?;
    Ok((StatusCode::CREATED, envelope(view)))
}

#[derive(Serialize)]
pub struct CreditCardsResponse {
    credit_cards: Vec<CreditCardResponse>,
}

/// `GET /credit-cards` (protected): the user's cards, active first.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<CreditCardsResponse>, ApiError> {
    let views = ListCreditCardsUseCase::new(CreditCardRepository::new(state.pool.clone()))
        .execute(auth_user.id)
        .await?;
    Ok(Json(CreditCardsResponse {
        credit_cards: views.into_iter().map(Into::into).collect(),
    }))
}

/// Every field is optional: only what is present changes. New days only
/// affect the invoices still open.
#[derive(Deserialize, Validate)]
pub struct UpdateRequest {
    #[serde(default, deserialize_with = "trimmed_opt")]
    #[validate(length(min = 1, max = MAX_NAME_LEN, code = "name_length"))]
    name: Option<String>,
    #[validate(range(min = 1, max = 31, code = "day_range"))]
    closing_day: Option<u32>,
    #[validate(range(min = 1, max = 31, code = "day_range"))]
    due_day: Option<u32>,
}

/// `PATCH /credit-cards/{id}` (protected): renames and/or changes the days.
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateRequest>,
) -> Result<Json<CreditCardEnvelope>, ApiError> {
    let view = UpdateCreditCardUseCase::new(
        CreditCardRepository::new(state.pool.clone()),
        InstallmentPurchaseRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        parse_id(&id)?,
        CreditCardChanges {
            name: request.name,
            closing_day: request.closing_day,
            due_day: request.due_day,
        },
        state.clock.today(),
    )
    .await?;
    Ok(envelope(view))
}

/// `POST /credit-cards/{id}/deactivate` (protected): only when no invoice has
/// an amount left to pay (`409 credit_card_has_remaining_invoice`).
pub async fn deactivate(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<CreditCardEnvelope>, ApiError> {
    let view = DeactivateCreditCardUseCase::new(
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, parse_id(&id)?, state.clock.today())
    .await?;
    Ok(envelope(view))
}

/// `POST /credit-cards/{id}/reactivate` (protected).
pub async fn reactivate(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<CreditCardEnvelope>, ApiError> {
    let view = ReactivateCreditCardUseCase::new(CreditCardRepository::new(state.pool.clone()))
        .execute(auth_user.id, parse_id(&id)?)
        .await?;
    Ok(envelope(view))
}

/// Where an invoice is: `open` receives purchases (up to and including the
/// closing date), `closed` is waiting for its due date, `paid` is from the
/// due date on.
fn state_name(state: InvoiceState) -> &'static str {
    match state {
        InvoiceState::Open => "open",
        InvoiceState::Closed => "closed",
        InvoiceState::Paid => "paid",
    }
}

/// The dates and state of the invoice a purchase belongs to.
#[derive(Serialize)]
pub struct InvoiceCycleResponse {
    /// `YYYY-MM`: the month the invoice closes, which identifies it.
    closing_month: String,
    closing_date: NaiveDate,
    due_date: NaiveDate,
    /// `YYYY-MM`: the month of the due date, where the invoice's purchases
    /// count in reports and budgets (the Reference Month).
    reference_month: String,
    state: &'static str,
}

impl From<InvoiceCycle> for InvoiceCycleResponse {
    fn from(cycle: InvoiceCycle) -> Self {
        Self {
            closing_month: cycle.closing_month.to_string(),
            closing_date: cycle.closing_date,
            due_date: cycle.due_date,
            reference_month: cycle.reference_month().to_string(),
            state: state_name(cycle.state),
        }
    }
}

/// The public view of an invoice.
#[derive(Serialize)]
pub struct InvoiceResponse {
    credit_card_id: CreditCardId,
    #[serde(flatten)]
    cycle: InvoiceCycleResponse,
    /// Cents: the sum of the purchases, Installments and Charges in the
    /// invoice, plus the Carried Balance it received.
    total_cents: i64,
    /// Cents that left the balance for this invoice: the Early Payments and,
    /// once the invoice is paid, the due-date payment (automatic, or the amount
    /// the user corrected it to).
    paid_cents: i64,
    /// Cents still to pay: the total less the Early Payments. Zero once the
    /// invoice is paid (the due date debits the rest by itself).
    remaining_cents: i64,
    /// Cents of Charges (interest, penalty, IOF) recorded in the invoice; part
    /// of `total_cents`.
    charges_cents: i64,
    /// Cents of Carried Balance received from the previous invoice; part of
    /// `total_cents`. Not an expense.
    carried_balance_cents: i64,
    /// Cents left unpaid at the due date that moved on to the next invoice as
    /// its Carried Balance.
    carried_forward_cents: i64,
    /// Whether the invoice's payment can be corrected to the amount really paid
    /// at the bank.
    payment_correction: PaymentCorrectionResponse,
}

/// Whether, and up to how much, the due-date payment of an invoice can be
/// corrected (UC20).
#[derive(Serialize)]
pub struct PaymentCorrectionResponse {
    /// `not_due` (the due date has not come), `nothing_to_pay` (Early Payments
    /// covered it all), `editable` (the invoice is paid and the next one, which
    /// receives the Carried Balance, is not) or `locked` (the next invoice is
    /// paid too).
    state: &'static str,
    /// Cents the user said they paid, once they corrected the payment.
    corrected_cents: Option<i64>,
    /// Cents: the most the payment can be corrected to (what the Early
    /// Payments left unpaid).
    max_cents: i64,
}

impl From<Invoice> for InvoiceResponse {
    fn from(invoice: Invoice) -> Self {
        let correction = invoice.payment_correction;
        Self {
            credit_card_id: invoice.card_id,
            cycle: invoice.cycle.into(),
            total_cents: invoice.total_cents,
            paid_cents: invoice.paid_cents,
            remaining_cents: invoice.remaining_cents,
            charges_cents: invoice.charges_cents,
            carried_balance_cents: invoice.carried_in_cents,
            carried_forward_cents: invoice.carried_out_cents,
            payment_correction: PaymentCorrectionResponse {
                state: match correction.state {
                    CorrectionState::NotDue => "not_due",
                    CorrectionState::NothingToPay => "nothing_to_pay",
                    CorrectionState::Editable => "editable",
                    CorrectionState::Locked => "locked",
                },
                corrected_cents: correction.corrected_cents,
                max_cents: correction.max_cents,
            },
        }
    }
}

/// The Carried Balance an invoice received, shown as an item of it.
#[derive(Serialize)]
pub struct CarriedBalanceResponse {
    amount_cents: i64,
    /// `YYYY-MM`: the closing month of the invoice the balance came from.
    from_closing_month: String,
}

#[derive(Serialize)]
pub struct InvoicesResponse {
    invoices: Vec<InvoiceResponse>,
}

/// `GET /credit-cards/{id}/invoices` (protected): the card's invoices, oldest
/// first: those with purchases plus the first open one. They are computed on
/// every read, never stored.
pub async fn invoices(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<InvoicesResponse>, ApiError> {
    let invoices = ListInvoicesUseCase::new(
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, parse_id(&id)?, state.clock.today())
    .await?;
    Ok(Json(InvoicesResponse {
        invoices: invoices.into_iter().map(Into::into).collect(),
    }))
}

#[derive(Serialize)]
pub struct InvoiceDetailResponse {
    invoice: InvoiceResponse,
    /// Newest first.
    purchases: Vec<CardPurchaseResponse>,
    /// The Installments of installment purchases in this invoice (the "3/10"
    /// lines), newest purchase first.
    installments: Vec<InvoiceInstallmentResponse>,
    /// The Charges (interest, penalty, IOF) recorded in this invoice, newest
    /// first. They are expenses of "Juros e encargos".
    charges: Vec<ChargeResponse>,
    /// The Carried Balance this invoice received from the previous one, shown
    /// as an item of it; `null` when there is none. It is not an expense.
    carried_balance: Option<CarriedBalanceResponse>,
    /// The Early Payments and the payment of the due date (when something is
    /// left unpaid): automatic, or corrected by the user. Oldest first. None is
    /// an expense.
    payments: Vec<InvoicePaymentResponse>,
}

/// `GET /credit-cards/{id}/invoices/{closing_month}` (protected): one invoice
/// (`closing_month` is `YYYY-MM`) with its purchases.
pub async fn invoice(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((id, closing_month)): Path<(String, String)>,
) -> Result<Json<InvoiceDetailResponse>, ApiError> {
    let card_id = parse_id(&id)?;
    let month = Month::parse(&closing_month).ok_or(ApiError::InvoiceNotFound)?;
    let today = state.clock.today();
    let detail = GetInvoiceUseCase::new(
        CreditCardRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        InstallmentPurchaseRepository::new(state.pool.clone()),
        InvoiceChargeRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, card_id, month, today)
    .await?;
    let carried_balance = (detail.invoice.carried_in_cents > 0).then(|| CarriedBalanceResponse {
        amount_cents: detail.invoice.carried_in_cents,
        from_closing_month: month.prev().to_string(),
    });
    Ok(Json(InvoiceDetailResponse {
        invoice: detail.invoice.into(),
        charges: detail
            .charges
            .into_iter()
            .map(|view| ChargeResponse::new(view, today))
            .collect(),
        carried_balance,
        purchases: detail
            .purchases
            .into_iter()
            .map(|view| CardPurchaseResponse::new(view, today))
            .collect(),
        installments: detail.installments.into_iter().map(Into::into).collect(),
        payments: detail
            .payments
            .iter()
            .map(|payment| InvoicePaymentResponse::new(payment, today))
            .collect(),
    }))
}
