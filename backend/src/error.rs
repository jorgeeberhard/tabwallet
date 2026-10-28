//! The HTTP error type and its stable JSON shape.
//!
//! Every error response looks like
//! `{"error": {"code": "email_taken", "message": "...", "fields": {...}}}`.
//! The front-end switches on `code` (stable) and shows its own Portuguese
//! text; `message` is English, for logs and developers; `fields` appears only
//! for validation errors and maps a field name to a rule code.

use std::collections::BTreeMap;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use validator::ValidationErrors;

use crate::{
    ids::CategoryId,
    use_cases::{
        AuthError, BalanceError, BalanceSeriesError, CardProblem, CardPurchaseError, CategoryError,
        CategoryProblem, CreditCardError, RecurringError, TransactionError,
    },
};

#[derive(Debug)]
pub enum ApiError {
    /// The body is not the JSON the route expects.
    InvalidRequest,
    /// The body parsed but broke a validation rule.
    Validation(BTreeMap<String, String>),
    EmailTaken,
    InvalidCredentials,
    /// The current password sent to confirm a sensitive change is wrong.
    InvalidCurrentPassword,
    /// The password-reset link is unknown, expired or already used.
    InvalidResetToken,
    /// No valid session.
    Unauthenticated,
    /// No such category for this user.
    CategoryNotFound,
    /// An active category of the same type already has this name.
    CategoryNameTaken,
    /// A deactivated category has this name; carries its id so the client can
    /// offer to reactivate it.
    CategoryInactiveExists(CategoryId),
    /// "Juros e encargos" still has Charges, so it cannot be deactivated.
    CategoryHasCharges,
    /// No such transaction for this user.
    TransactionNotFound,
    /// The Opening Balance was already informed; it is informed once.
    OpeningBalanceAlreadySet,
    /// No such credit card for this user.
    CreditCardNotFound,
    /// The card has no such invoice.
    InvoiceNotFound,
    /// Another card of the user has this name.
    CreditCardNameTaken,
    /// An invoice of the card still has an amount to pay, so the card cannot
    /// be deactivated.
    CreditCardHasRemainingInvoice,
    /// No such card purchase for this user.
    CardPurchaseNotFound,
    /// The card purchase is in a paid invoice and cannot be edited or deleted.
    InvoicePaid,
    /// No such recurring Transaction for this user.
    RecurringTransactionNotFound,
    /// The recurring Transaction has no Occurrence in that month.
    OccurrenceNotFound,
    /// The recurring Transaction was already ended and cannot change.
    RecurringTransactionEnded,
    /// Something unexpected; the cause is logged, never sent to the client.
    Internal(String),
}

impl ApiError {
    pub fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(errors: ValidationErrors) -> Self {
        let fields = errors
            .field_errors()
            .into_iter()
            .filter_map(|(field, rules)| {
                rules
                    .first()
                    .map(|rule| (field.to_string(), rule.code.to_string()))
            })
            .collect();
        Self::Validation(fields)
    }
}

impl From<AuthError> for ApiError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::EmailTaken => Self::EmailTaken,
            AuthError::InvalidCredentials => Self::InvalidCredentials,
            AuthError::InvalidCurrentPassword => Self::InvalidCurrentPassword,
            AuthError::CurrentPasswordRequired => Self::Validation(BTreeMap::from([(
                "current_password".to_string(),
                "required".to_string(),
            )])),
            AuthError::InvalidResetToken => Self::InvalidResetToken,
            AuthError::Unauthenticated => Self::Unauthenticated,
            AuthError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl From<CategoryError> for ApiError {
    fn from(error: CategoryError) -> Self {
        match error {
            CategoryError::NotFound => Self::CategoryNotFound,
            CategoryError::NameTaken => Self::CategoryNameTaken,
            CategoryError::InactiveNameExists(id) => Self::CategoryInactiveExists(id),
            CategoryError::HasCharges => Self::CategoryHasCharges,
            CategoryError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl From<TransactionError> for ApiError {
    fn from(error: TransactionError) -> Self {
        match error {
            TransactionError::NotFound => Self::TransactionNotFound,
            TransactionError::InvalidCategory(problem) => {
                let rule = match problem {
                    CategoryProblem::NotFound => "category_not_found",
                    CategoryProblem::TypeMismatch => "category_type_mismatch",
                    CategoryProblem::Inactive => "category_inactive",
                };
                Self::Validation(BTreeMap::from([(
                    "category_id".to_string(),
                    rule.to_string(),
                )]))
            }
            TransactionError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl From<BalanceError> for ApiError {
    fn from(error: BalanceError) -> Self {
        match error {
            BalanceError::OpeningBalanceAlreadySet => Self::OpeningBalanceAlreadySet,
            BalanceError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl From<BalanceSeriesError> for ApiError {
    fn from(error: BalanceSeriesError) -> Self {
        match error {
            BalanceSeriesError::InvalidPeriod { field, rule } => {
                Self::Validation(BTreeMap::from([(field.to_string(), rule.to_string())]))
            }
            BalanceSeriesError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl From<CreditCardError> for ApiError {
    fn from(error: CreditCardError) -> Self {
        match error {
            CreditCardError::NotFound => Self::CreditCardNotFound,
            CreditCardError::InvoiceNotFound => Self::InvoiceNotFound,
            CreditCardError::NameTaken => Self::CreditCardNameTaken,
            CreditCardError::HasRemainingInvoice => Self::CreditCardHasRemainingInvoice,
            CreditCardError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl From<CardPurchaseError> for ApiError {
    fn from(error: CardPurchaseError) -> Self {
        let field = |name: &str, rule: &str| {
            Self::Validation(BTreeMap::from([(name.to_string(), rule.to_string())]))
        };
        match error {
            CardPurchaseError::NotFound => Self::CardPurchaseNotFound,
            CardPurchaseError::InvalidCategory(problem) => field(
                "category_id",
                match problem {
                    CategoryProblem::NotFound => "category_not_found",
                    CategoryProblem::TypeMismatch => "category_type_mismatch",
                    CategoryProblem::Inactive => "category_inactive",
                },
            ),
            CardPurchaseError::InvalidCard(problem) => field(
                "credit_card_id",
                match problem {
                    CardProblem::NotFound => "credit_card_not_found",
                    CardProblem::Inactive => "credit_card_inactive",
                },
            ),
            CardPurchaseError::InvoicePaid => Self::InvoicePaid,
            CardPurchaseError::DateInPaidInvoice => field("date", "invoice_paid"),
            CardPurchaseError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl From<RecurringError> for ApiError {
    fn from(error: RecurringError) -> Self {
        match error {
            RecurringError::NotFound => Self::RecurringTransactionNotFound,
            RecurringError::OccurrenceNotFound => Self::OccurrenceNotFound,
            RecurringError::Ended => Self::RecurringTransactionEnded,
            RecurringError::InvalidCategory(problem) => {
                TransactionError::InvalidCategory(problem).into()
            }
            RecurringError::Invalid { field, rule } => {
                Self::Validation(BTreeMap::from([(field.to_string(), rule.to_string())]))
            }
            RecurringError::Internal(cause) => Self::Internal(cause),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Extra top-level members of the `error` object, for the few errors
        // whose client needs more than the code.
        let mut extra = None;
        let (status, code, message, fields) = match self {
            Self::InvalidRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request body is not valid JSON for this route.",
                None,
            ),
            Self::Validation(fields) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "One or more fields are invalid.",
                Some(fields),
            ),
            Self::EmailTaken => (
                StatusCode::CONFLICT,
                "email_taken",
                "An account with this e-mail already exists.",
                None,
            ),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Wrong e-mail or password.",
                None,
            ),
            // 403, not 401: the session is fine, and the front-end treats 401
            // as "signed out".
            Self::InvalidCurrentPassword => (
                StatusCode::FORBIDDEN,
                "invalid_current_password",
                "The current password is wrong.",
                None,
            ),
            Self::InvalidResetToken => (
                StatusCode::BAD_REQUEST,
                "invalid_reset_token",
                "The password reset link is invalid, expired or already used.",
                None,
            ),
            Self::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Sign in to continue.",
                None,
            ),
            Self::CategoryNotFound => (
                StatusCode::NOT_FOUND,
                "category_not_found",
                "There is no such category.",
                None,
            ),
            Self::CategoryNameTaken => (
                StatusCode::CONFLICT,
                "category_name_taken",
                "A category of this type already has this name.",
                None,
            ),
            Self::CategoryInactiveExists(id) => {
                extra = Some(("category_id", json!(id)));
                (
                    StatusCode::CONFLICT,
                    "category_inactive_exists",
                    "A deactivated category of this type has this name; reactivate it instead.",
                    None,
                )
            }
            Self::CategoryHasCharges => (
                StatusCode::CONFLICT,
                "category_has_charges",
                "This category still has charges and cannot be deactivated.",
                None,
            ),
            Self::TransactionNotFound => (
                StatusCode::NOT_FOUND,
                "transaction_not_found",
                "There is no such transaction.",
                None,
            ),
            Self::OpeningBalanceAlreadySet => (
                StatusCode::CONFLICT,
                "opening_balance_already_set",
                "The opening balance was already informed and cannot be changed.",
                None,
            ),
            Self::CreditCardNotFound => (
                StatusCode::NOT_FOUND,
                "credit_card_not_found",
                "There is no such credit card.",
                None,
            ),
            Self::InvoiceNotFound => (
                StatusCode::NOT_FOUND,
                "invoice_not_found",
                "The credit card has no such invoice.",
                None,
            ),
            Self::CreditCardNameTaken => (
                StatusCode::CONFLICT,
                "credit_card_name_taken",
                "A credit card with this name already exists.",
                None,
            ),
            Self::CreditCardHasRemainingInvoice => (
                StatusCode::CONFLICT,
                "credit_card_has_remaining_invoice",
                "An invoice of this credit card still has an amount to pay.",
                None,
            ),
            Self::CardPurchaseNotFound => (
                StatusCode::NOT_FOUND,
                "card_purchase_not_found",
                "There is no such card purchase.",
                None,
            ),
            Self::InvoicePaid => (
                StatusCode::CONFLICT,
                "invoice_paid",
                "The purchase is in a paid invoice and cannot be changed.",
                None,
            ),
            Self::RecurringTransactionNotFound => (
                StatusCode::NOT_FOUND,
                "recurring_transaction_not_found",
                "There is no such recurring transaction.",
                None,
            ),
            Self::OccurrenceNotFound => (
                StatusCode::NOT_FOUND,
                "occurrence_not_found",
                "The recurring transaction has no occurrence in that month.",
                None,
            ),
            Self::RecurringTransactionEnded => (
                StatusCode::CONFLICT,
                "recurring_transaction_ended",
                "The recurring transaction has ended and cannot be changed.",
                None,
            ),
            Self::Internal(cause) => {
                tracing::error!("internal error: {cause}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Something went wrong.",
                    None,
                )
            }
        };
        let mut error = json!({ "code": code, "message": message });
        if let Some(fields) = fields {
            error["fields"] = json!(fields);
        }
        if let Some((key, value)) = extra {
            error[key] = value;
        }
        (status, Json(json!({ "error": error }))).into_response()
    }
}
