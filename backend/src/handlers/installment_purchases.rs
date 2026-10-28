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
        InstallmentPurchaseView, InstallmentView, UpdateInstallmentPurchaseUseCase,
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
}

impl From<InstallmentView> for InstallmentResponse {
    fn from(view: InstallmentView) -> Self {
        Self {
            id: view.installment.id,
            number: view.installment.number,
            amount_cents: view.installment.amount_cents,
            reference_month: view.invoice.reference_month().to_string(),
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
    /// The Installments still on record, by number.
    installments: Vec<InstallmentResponse>,
    /// The date is after today (America/Sao_Paulo).
    is_projection: bool,
    created_at: DateTime<Utc>,
}

impl InstallmentPurchaseResponse {
    pub fn new(view: InstallmentPurchaseView, today: NaiveDate) -> Self {
        let has_paid_installments = view.has_paid_installments();
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
            installments: view.installments.into_iter().map(Into::into).collect(),
            is_projection: purchase.date > today,
            created_at: purchase.created_at,
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
