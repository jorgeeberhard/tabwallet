use axum::{
    Json,
    extract::{FromRequest, Request},
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::ApiError;

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
