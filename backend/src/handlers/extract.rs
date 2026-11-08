use axum::{
    Json,
    extract::{FromRequest, Request},
};
use serde::{Deserialize, de::DeserializeOwned};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::ApiError,
    ids::{
        CardPurchaseId, CategoryId, ChargeId, CreditCardId, InstallmentPurchaseId,
        RecurringTransactionId, TransactionId,
    },
};

/// For `#[serde(deserialize_with = "trimmed")]`: surrounding spaces are a
/// copy-and-paste accident, not part of an e-mail or a name.
pub(super) fn trimmed<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    String::deserialize(deserializer).map(|value| value.trim().to_string())
}

/// A JSON body that is parsed and then checked with the `validator` crate.
/// Failures become the API's stable error responses (400 or 422).
pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(request, state)
            .await
            .map_err(|_| ApiError::InvalidRequest)?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Like [`trimmed`], for an optional field of an edit request.
pub(super) fn trimmed_opt<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer).map(|value| value.map(|v| v.trim().to_string()))
}

/// An id that arrives as text in a path (`/card-purchases/{id}`). A malformed
/// one names a resource that does not exist, so it fails with that resource's
/// own `404`.
pub(super) trait PathId: Sized {
    fn from_uuid(uuid: Uuid) -> Self;
    fn not_found() -> ApiError;
}

macro_rules! path_ids {
    ($($id:ident => $not_found:ident),+ $(,)?) => {
        $(impl PathId for $id {
            fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            fn not_found() -> ApiError {
                ApiError::$not_found
            }
        })+
    };
}

path_ids! {
    CardPurchaseId => CardPurchaseNotFound,
    CategoryId => CategoryNotFound,
    ChargeId => ChargeNotFound,
    CreditCardId => CreditCardNotFound,
    InstallmentPurchaseId => InstallmentPurchaseNotFound,
    RecurringTransactionId => RecurringTransactionNotFound,
    TransactionId => TransactionNotFound,
}

/// Reads a path id: `parse_id(&id)?` takes the id type the caller needs.
pub(super) fn parse_id<T: PathId>(raw: &str) -> Result<T, ApiError> {
    Uuid::parse_str(raw)
        .map(T::from_uuid)
        .map_err(|_| T::not_found())
}
