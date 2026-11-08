//! A small in-memory limiter for the two operations that guessing or spam can
//! abuse without a session: logging in and asking for a password-reset e-mail.
//!
//! It counts attempts per key (the e-mail address, lowercased) in a sliding
//! window. The counters live in the memory of one API instance, so with several
//! instances the effective limit is per instance; that is enough for the MVP
//! and needs no extra service. A restart forgets them.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Duration, Utc};

/// More keys than this and the limiter drops the ones whose window is over.
const PRUNE_ABOVE_KEYS: usize = 10_000;

#[derive(Clone)]
pub struct RateLimiter {
    max_attempts: usize,
    window: Duration,
    attempts: Arc<Mutex<HashMap<String, Vec<DateTime<Utc>>>>>,
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window: Duration) -> Self {
        Self {
            max_attempts,
            window,
            attempts: Arc::default(),
        }
    }

    /// `Ok` while the key is under the limit; otherwise `Err` with the seconds
    /// until the oldest counted attempt leaves the window.
    pub fn check(&self, key: &str, now: DateTime<Utc>) -> Result<(), u64> {
        let mut attempts = self.attempts.lock().unwrap();
        let Some(times) = attempts.get_mut(key) else {
            return Ok(());
        };
        times.retain(|time| *time + self.window > now);
        if times.len() < self.max_attempts {
            return Ok(());
        }
        let oldest = times.iter().min().copied().unwrap_or(now);
        let wait = (oldest + self.window - now).num_seconds().max(1);
        Err(wait as u64)
    }

    /// Counts one attempt for the key.
    pub fn record(&self, key: &str, now: DateTime<Utc>) {
        let mut attempts = self.attempts.lock().unwrap();
        if attempts.len() > PRUNE_ABOVE_KEYS {
            attempts.retain(|_, times| times.iter().any(|time| *time + self.window > now));
        }
        let times = attempts.entry(key.to_string()).or_default();
        times.retain(|time| *time + self.window > now);
        times.push(now);
    }

    /// Forgets the key's attempts (after a successful login).
    pub fn clear(&self, key: &str) {
        self.attempts.lock().unwrap().remove(key);
    }
}

/// The limiters of the API. Login counts failed attempts: 5 per 15 minutes
/// for an e-mail address. Password-reset requests count every request: 3 per
/// hour for an e-mail address.
#[derive(Clone)]
pub struct RateLimits {
    pub login: RateLimiter,
    pub password_reset: RateLimiter,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            login: RateLimiter::new(5, Duration::minutes(15)),
            password_reset: RateLimiter::new(3, Duration::hours(1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minutes: i64) -> DateTime<Utc> {
        "2026-01-15T12:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::minutes(minutes)
    }

    #[test]
    fn blocks_after_the_maximum_and_says_how_long_to_wait() {
        let limiter = RateLimiter::new(2, Duration::minutes(10));
        assert_eq!(limiter.check("ana", at(0)), Ok(()));
        limiter.record("ana", at(0));
        limiter.record("ana", at(3));

        assert_eq!(limiter.check("ana", at(4)), Err(6 * 60));
        assert_eq!(limiter.check("bia", at(4)), Ok(()));
    }

    #[test]
    fn attempts_leave_the_window_and_the_key_is_free_again() {
        let limiter = RateLimiter::new(2, Duration::minutes(10));
        limiter.record("ana", at(0));
        limiter.record("ana", at(3));

        assert_eq!(limiter.check("ana", at(10)), Ok(()));
    }

    #[test]
    fn clear_forgets_the_attempts() {
        let limiter = RateLimiter::new(1, Duration::minutes(10));
        limiter.record("ana", at(0));
        assert!(limiter.check("ana", at(1)).is_err());

        limiter.clear("ana");

        assert_eq!(limiter.check("ana", at(1)), Ok(()));
    }
}
