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
    credit_cards::InvoiceCycleResponse,
    extract::{ValidatedJson, trimmed, trimmed_opt},
    transactions::{MAX_AMOUNT_CENTS, MAX_DESCRIPTION_LEN, TransactionCategory},
};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::{CategoryId, CreditCardId, InstallmentId, InstallmentPurchaseId},
    ledger::cards::{MAX_INSTALLMENTS, MIN_INSTALLMENTS},
    repositories::{
        card_purchases::CardPurchaseRepository,
        categories::CategoryRepository,
        credit_cards::CreditCardRepository,
        installment_purchases::{
            InstallmentPurchaseChanges, InstallmentPurchaseRepository, NewInstallmentPurchase,
        },
    },
    use_cases::{
        CancelInstallmentPurchaseUseCase, CancelOutcome, CreateInstallmentPurchaseUseCase,
        InstallmentPurchaseView, InstallmentView, InvoiceInstallment,
        UpdateInstallmentPurchaseUseCase,
    },
};

/// The limits on the number of Installments, as the signed numbers the
/// requests carry (so that a negative count is a validation error).
const MIN_COUNT: i64 = MIN_INSTALLMENTS as i64;
const MAX_COUNT: i64 = MAX_INSTALLMENTS as i64;

/// One Installment, with the invoice it is in.
#[derive(Serialize)]
pub struct InstallmentResponse {
    id: InstallmentId,
    /// 1 for the first Installment, up to `installment_count`.
    number: i32,
    /// Cents, always greater than zero.
    amount_cents: i64,
    /// The invoice the Installment is in. Worked out on every read: its
    /// state changes with the clock.
    invoice: InvoiceCycleResponse,
    /// The month the Installment counts in for reports and budgets: the
    /// month of the invoice's due date.
    reference_month: String,
    /// It was brought to the open invoice by an Installment Anticipation
    ///, which cannot be undone.
    anticipated: bool,
    /// The day it was anticipated, `YYYY-MM-DD`; `null` if it was not.
    anticipated_on: Option<NaiveDate>,
    /// Cents of Discount it received when anticipated; zero otherwise.
    discount_cents: i64,
    /// Cents before the Discount: `amount_cents` plus `discount_cents`.
    original_amount_cents: i64,
    /// `YYYY-MM`: the invoice an anticipated Installment came from.
    original_invoice_month: Option<String>,
}

impl From<InstallmentView> for InstallmentResponse {
    fn from(view: InstallmentView) -> Self {
        Self {
            id: view.installment.id,
            number: view.installment.number,
            amount_cents: view.installment.amount_cents,
            reference_month: view.invoice.reference_month.to_string(),
            anticipated: view.installment.is_anticipated(),
            anticipated_on: view.installment.anticipated_on,
            discount_cents: view.installment.discount_cents,
            original_amount_cents: view.installment.original_amount_cents(),
            original_invoice_month: view
                .installment
                .original_invoice_month()
                .map(|month| month.to_string()),
            invoice: view.invoice.into(),
        }
    }
}

/// The public view of an installment purchase.
#[derive(Serialize)]
pub struct InstallmentPurchaseResponse {
    id: InstallmentPurchaseId,
    /// Cents: what the user informed, already with interest if any. It is an
    /// expense.
    total_cents: i64,
    /// How many Installments the purchase was launched in.
    installment_count: i32,
    /// The day of the purchase, `YYYY-MM-DD`.
    date: NaiveDate,
    description: String,
    category: TransactionCategory,
    credit_card: PurchaseCard,
    /// The remaining Installments were cancelled after some had been paid;
    /// `installments` then only has the paid ones.
    cancelled: bool,
    /// Some Installment is in a paid invoice: only the Category can change
    /// and the remaining Installments can be cancelled.
    has_paid_installments: bool,
    /// Some Installment was anticipated, which cannot be undone: like
    /// with paid ones, only the Category can change and the remaining
    /// Installments can be cancelled.
    has_anticipated_installments: bool,
    /// Cents of Discount the purchase received in anticipations. The expense
    /// in its Category is the sum of the Installments, which already
    /// excludes it; `total_cents` stays what the user informed.
    discount_cents: i64,
    /// The Installments still on record, by number.
    installments: Vec<InstallmentResponse>,
    /// The date is after today (America/Sao_Paulo).
    is_projection: bool,
    created_at: DateTime<Utc>,
}

impl InstallmentPurchaseResponse {
    pub fn new(view: InstallmentPurchaseView, today: NaiveDate) -> Self {
        let has_paid_installments = view.has_paid_installments();
        let has_anticipated_installments = view.has_anticipated_installments();
        let discount_cents = view.discount_cents();
        let purchase = view.purchase;
        Self {
            id: purchase.id,
            total_cents: purchase.total_cents,
            installment_count: purchase.installment_count,
            date: purchase.date,
            description: purchase.description,
            category: TransactionCategory::new(
                purchase.category_id,
                purchase.category_name,
                "expense",
                purchase.category_active,
            ),
            credit_card: PurchaseCard::new(
                purchase.credit_card_id,
                purchase.credit_card_name,
                purchase.credit_card_active,
            ),
            cancelled: purchase.cancelled,
            has_paid_installments,
            has_anticipated_installments,
            discount_cents,
            installments: view.installments.into_iter().map(Into::into).collect(),
            is_projection: purchase.date > today,
            created_at: purchase.created_at,
        }
    }
}

/// One Installment as an item of an invoice: its own amount and invoice with
/// the data of the purchase it belongs to, for the "3/10" lines.
#[derive(Serialize)]
pub struct InvoiceInstallmentResponse {
    id: InstallmentId,
    installment_purchase_id: InstallmentPurchaseId,
    /// 1 for the first Installment, up to `installment_count`.
    number: i32,
    installment_count: i32,
    /// Cents, always greater than zero: this Installment's share.
    amount_cents: i64,
    /// Cents: the whole purchase.
    total_cents: i64,
    /// The day of the purchase, `YYYY-MM-DD`.
    date: NaiveDate,
    description: String,
    category: TransactionCategory,
    credit_card: PurchaseCard,
    invoice: InvoiceCycleResponse,
    reference_month: String,
    /// Some Installment of the purchase is in a paid invoice, so the purchase
    /// can only have its Category changed or be cancelled.
    purchase_has_paid_installments: bool,
    /// Some Installment of the purchase was anticipated.
    purchase_has_anticipated_installments: bool,
    /// This Installment was brought to this invoice by an Installment
    /// Anticipation.
    anticipated: bool,
    /// Cents of Discount it received when anticipated; zero otherwise.
    discount_cents: i64,
    /// Cents before the Discount.
    original_amount_cents: i64,
    /// `YYYY-MM`: the invoice an anticipated Installment came from.
    original_invoice_month: Option<String>,
}

impl From<InvoiceInstallment> for InvoiceInstallmentResponse {
    fn from(item: InvoiceInstallment) -> Self {
        let purchase = item.purchase;
        Self {
            id: item.installment.installment.id,
            installment_purchase_id: purchase.id,
            number: item.installment.installment.number,
            installment_count: purchase.installment_count,
            amount_cents: item.installment.installment.amount_cents,
            total_cents: purchase.total_cents,
            date: purchase.date,
            description: purchase.description,
            category: TransactionCategory::new(
                purchase.category_id,
                purchase.category_name,
                "expense",
                purchase.category_active,
            ),
            credit_card: PurchaseCard::new(
                purchase.credit_card_id,
                purchase.credit_card_name,
                purchase.credit_card_active,
            ),
            reference_month: item.installment.invoice.reference_month.to_string(),
            anticipated: item.installment.installment.is_anticipated(),
            discount_cents: item.installment.installment.discount_cents,
            original_amount_cents: item.installment.installment.original_amount_cents(),
            original_invoice_month: item
                .installment
                .installment
                .original_invoice_month()
                .map(|month| month.to_string()),
            invoice: item.installment.invoice.into(),
            purchase_has_paid_installments: item.purchase_has_paid_installments,
            purchase_has_anticipated_installments: item.purchase_has_anticipated_installments,
        }
    }
}

#[derive(Serialize)]
pub struct InstallmentPurchaseEnvelope {
    installment_purchase: InstallmentPurchaseResponse,
}

#[derive(Deserialize, Validate)]
pub struct CreateRequest {
    credit_card_id: CreditCardId,
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    total_cents: i64,
    #[validate(range(
        min = MIN_COUNT,
        max = MAX_COUNT,
        code = "installment_count_range"
    ))]
    installment_count: i64,
    date: NaiveDate,
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: String,
    category_id: CategoryId,
}

/// `POST /installment-purchases` (protected): records a purchase with a
/// credit card in 2 to 48 Installments. The total is split in exact cents
/// (the leftover on the first Installment) and each Installment goes to a
/// consecutive invoice, the first to the one that would take the purchase
/// paid in full. It does not change the Current Balance.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateRequest>,
) -> Result<(StatusCode, Json<InstallmentPurchaseEnvelope>), ApiError> {
    let today = state.clock.today();
    let view = CreateInstallmentPurchaseUseCase::new(
        InstallmentPurchaseRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        NewInstallmentPurchase {
            credit_card_id: request.credit_card_id,
            total_cents: request.total_cents,
            installment_count: request.installment_count as u32,
            date: request.date,
            description: request.description,
            category_id: request.category_id,
        },
        today,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(InstallmentPurchaseEnvelope {
            installment_purchase: InstallmentPurchaseResponse::new(view, today),
        }),
    ))
}

/// Every field is optional: only what is present changes. The card cannot be
/// changed.
#[derive(Deserialize, Validate)]
pub struct UpdateRequest {
    #[validate(range(min = 1, max = MAX_AMOUNT_CENTS, code = "amount_range"))]
    total_cents: Option<i64>,
    #[validate(range(
        min = MIN_COUNT,
        max = MAX_COUNT,
        code = "installment_count_range"
    ))]
    installment_count: Option<i64>,
    date: Option<NaiveDate>,
    #[serde(default, deserialize_with = "trimmed_opt")]
    #[validate(length(min = 1, max = MAX_DESCRIPTION_LEN, code = "description_length"))]
    description: Option<String>,
    category_id: Option<CategoryId>,
}

/// A malformed id is a purchase that does not exist.
fn parse_id(raw: &str) -> Result<InstallmentPurchaseId, ApiError> {
    Uuid::parse_str(raw)
        .map(InstallmentPurchaseId)
        .map_err(|_| ApiError::InstallmentPurchaseNotFound)
}

/// `PATCH /installment-purchases/{id}` (protected): edits total, number of
/// Installments, date, description and category. While no Installment is in
/// a paid invoice everything is recalculated and reallocated; otherwise only
/// the category can change (`409 installments_paid` for any other field).
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateRequest>,
) -> Result<Json<InstallmentPurchaseEnvelope>, ApiError> {
    let today = state.clock.today();
    let view = UpdateInstallmentPurchaseUseCase::new(
        InstallmentPurchaseRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
        CategoryRepository::new(state.pool.clone()),
    )
    .execute(
        auth_user.id,
        parse_id(&id)?,
        InstallmentPurchaseChanges {
            total_cents: request.total_cents,
            installment_count: request.installment_count.map(|count| count as u32),
            date: request.date,
            description: request.description,
            category_id: request.category_id,
        },
        today,
    )
    .await?;
    Ok(Json(InstallmentPurchaseEnvelope {
        installment_purchase: InstallmentPurchaseResponse::new(view, today),
    }))
}

/// `DELETE /installment-purchases/{id}` (protected). While no Installment is
/// in a paid invoice the whole purchase is removed (`204`). With some paid,
/// the remaining Installments are cancelled and the paid ones stay: the
/// answer is `200` with the purchase as it is now. When every Installment is
/// paid there is nothing to cancel (`409 invoice_paid`).
pub async fn delete(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;
    let today = state.clock.today();
    let outcome = CancelInstallmentPurchaseUseCase::new(
        InstallmentPurchaseRepository::new(state.pool.clone()),
        CardPurchaseRepository::new(state.pool.clone()),
        CreditCardRepository::new(state.pool.clone()),
    )
    .execute(auth_user.id, parse_id(&id)?, today)
    .await?;
    Ok(match outcome {
        CancelOutcome::Deleted => StatusCode::NO_CONTENT.into_response(),
        CancelOutcome::RemainingCancelled(view) => Json(InstallmentPurchaseEnvelope {
            installment_purchase: InstallmentPurchaseResponse::new(view, today),
        })
        .into_response(),
    })
}
