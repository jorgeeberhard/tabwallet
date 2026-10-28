//! Password-reset tokens.
//!
//! The token travels in the e-mailed link; the database keeps only its
//! SHA-256 hash. The token is random, not derived from anything, so a fast
//! hash is enough: there is nothing to brute-force.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A new random token: 64 hex characters, two random (v4) UUIDs from the
/// operating system's random generator.
pub fn generate() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// The value stored in the database for a token: lowercase hex SHA-256.
pub fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_long_and_never_repeat() {
        let first = generate();

        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first, generate());
    }

    #[test]
    fn the_hash_matches_the_known_sha256_and_is_not_the_token() {
        // SHA-256("abc"), from FIPS 180-2.
        assert_eq!(
            hash("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
