//! Authentication: password hashing, the session token and the session cookie.
//!
//! Protected routes sit behind [`require_auth`]. It reads the session cookie,
//! verifies the JWT and stores an [`AuthUser`] in the request; a handler then
//! asks for `AuthUser` as an argument and passes `auth_user.id` to its use
//! case, so every query is scoped to the signed-in user.

mod middleware;
pub mod password;
pub mod reset_token;
mod token;

pub use middleware::{AuthUser, require_auth};
pub use token::{TokenError, TokenService};

use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

/// Name of the session cookie. The Next.js proxy reads the same name.
pub const SESSION_COOKIE: &str = "tabwallet_session";

/// How long a session lasts after signing up or logging in.
const SESSION_TTL: Duration = Duration::days(7);

/// The JWT secret must be at least this long (a 256-bit key for HS256).
const MIN_SECRET_LEN: usize = 32;

/// Everything the API needs to create and check sessions.
#[derive(Clone)]
pub struct Auth {
    tokens: TokenService,
    cookie_secure: bool,
}

impl Auth {
    /// `cookie_secure` marks the session cookie `Secure` (HTTPS only); it must
    /// be true in production.
    pub fn new(jwt_secret: &str, cookie_secure: bool) -> Result<Self, String> {
        if jwt_secret.len() < MIN_SECRET_LEN {
            return Err(format!(
                "JWT_SECRET must have at least {MIN_SECRET_LEN} characters"
            ));
        }
        Ok(Self {
            tokens: TokenService::new(jwt_secret.as_bytes(), SESSION_TTL),
            cookie_secure,
        })
    }

    /// Reads `JWT_SECRET` (required) and `COOKIE_SECURE` (default `true`;
    /// set it to `false` only for local development over plain HTTP).
    pub fn from_env() -> Result<Self, String> {
        let secret = std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set")?;
        let cookie_secure = match std::env::var("COOKIE_SECURE").as_deref() {
            Err(_) | Ok("true") => true,
            Ok("false") => false,
            Ok(other) => return Err(format!("COOKIE_SECURE must be true or false, got {other}")),
        };
        Self::new(&secret, cookie_secure)
    }

    pub fn tokens(&self) -> &TokenService {
        &self.tokens
    }

    /// The `Set-Cookie` that starts a session: `HttpOnly` so scripts cannot
    /// read the token, `SameSite=Lax` so other sites cannot send it with a
    /// cross-site request.
    pub fn session_cookie(&self, token: String) -> Cookie<'static> {
        Cookie::build((SESSION_COOKIE, token))
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.cookie_secure)
            .path("/")
            .max_age(SESSION_TTL)
            .build()
    }

    /// The `Set-Cookie` that ends the session in the browser.
    pub fn removal_cookie(&self) -> Cookie<'static> {
        Cookie::build((SESSION_COOKIE, ""))
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.cookie_secure)
            .path("/")
            .max_age(Duration::ZERO)
            .build()
    }
}
