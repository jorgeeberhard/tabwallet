//! The session token: an HS256 JWT whose `sub` is the user id.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::ids::UserId;

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    iat: i64,
    exp: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("the token is invalid or expired")]
    Invalid,
    #[error("could not sign the token: {0}")]
    Signing(String),
}

#[derive(Clone)]
pub struct TokenService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    ttl: Duration,
}

impl TokenService {
    pub fn new(secret: &[u8], ttl: Duration) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            ttl,
        }
    }

    pub fn issue(&self, user: UserId) -> Result<String, TokenError> {
        let now = OffsetDateTime::now_utc();
        let claims = Claims {
            sub: user.0,
            iat: now.unix_timestamp(),
            exp: (now + self.ttl).unix_timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|error| TokenError::Signing(error.to_string()))
    }

    /// The user the token was issued to, if the signature is ours and the
    /// token has not expired.
    pub fn verify(&self, token: &str) -> Result<UserId, TokenError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 0;
        decode::<Claims>(token, &self.decoding, &validation)
            .map(|data| UserId(data.claims.sub))
            .map_err(|_| TokenError::Invalid)
    }
}
