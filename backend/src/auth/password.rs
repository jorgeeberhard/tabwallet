//! Argon2id password hashing.
//!
//! Hashing is CPU- and memory-heavy by design, so both functions run on the
//! blocking thread pool instead of stalling the async runtime.

use std::sync::LazyLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

/// Hashes a password to an Argon2id PHC string with a random salt.
pub async fn hash(password: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|error| format!("could not hash the password: {error}"))
    })
    .await
    .map_err(|error| format!("password hashing task failed: {error}"))?
}

/// Checks a password against a stored PHC string. A malformed stored hash
/// counts as a mismatch.
pub async fn verify(password: String, stored_hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&stored_hash)
            .map(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

/// A valid hash of a password nobody knows. Login verifies against it when the
/// e-mail is unknown, so the response takes as long as for a real account and
/// does not reveal which e-mails are registered.
static DECOY_HASH: LazyLock<String> = LazyLock::new(|| {
    Argon2::default()
        .hash_password(b"decoy password nobody can type in")
        .expect("hashing a constant cannot fail")
        .to_string()
});

pub fn decoy_hash() -> String {
    DECOY_HASH.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stores_an_argon2id_hash_that_only_the_right_password_matches() {
        let stored = hash("correct horse battery".to_string()).await.unwrap();

        assert!(stored.starts_with("$argon2id$"));
        assert!(!stored.contains("correct horse"));
        assert!(verify("correct horse battery".to_string(), stored.clone()).await);
        assert!(!verify("wrong horse battery".to_string(), stored).await);
    }

    #[tokio::test]
    async fn the_same_password_gets_a_different_hash_each_time() {
        let first = hash("correct horse battery".to_string()).await.unwrap();
        let second = hash("correct horse battery".to_string()).await.unwrap();

        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn a_malformed_stored_hash_never_matches() {
        assert!(!verify("anything".to_string(), "not a hash".to_string()).await);
    }
}
