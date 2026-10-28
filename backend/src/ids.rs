//! Identifiers shared by every layer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The id of a user. Every repository method that reads or writes a user's
/// data takes one, so data is always scoped to its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(pub Uuid);

/// The id of a Transaction (an income or expense outside the credit card).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct TransactionId(pub Uuid);

/// The id of a category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct CategoryId(pub Uuid);
