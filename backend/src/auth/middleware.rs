use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;

use super::SESSION_COOKIE;
use crate::{app::AppState, error::ApiError, ids::UserId};

/// The signed-in user of a request to a protected route.
///
/// Take it as a handler argument and scope every query to `id`.
#[derive(Clone, Copy, Debug)]
pub struct AuthUser {
    pub id: UserId,
}

/// Middleware for protected routes: rejects requests without a valid session
/// cookie with 401, and otherwise makes the user available as [`AuthUser`].
pub async fn require_auth(
    State(state): State<AppState>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = jar.get(SESSION_COOKIE).ok_or(ApiError::Unauthenticated)?;
    let id = state
        .auth
        .tokens()
        .verify(token.value())
        .map_err(|_| ApiError::Unauthenticated)?;
    request.extensions_mut().insert(AuthUser { id });
    Ok(next.run(request).await)
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Missing means a handler asked for `AuthUser` on a route that is not
        // behind `require_auth`: refuse rather than serve unscoped data.
        parts.extensions.get::<AuthUser>().copied().ok_or_else(|| {
            ApiError::internal("AuthUser requested on a route without the require_auth layer")
        })
    }
}
