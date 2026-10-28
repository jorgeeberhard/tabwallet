use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};

use crate::ids::{CategoryId, UserId};

/// A category belongs to exactly one type: it only classifies transactions of
/// that type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CategoryType {
    Income,
    Expense,
}

impl CategoryType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
        }
    }
}

impl TryFrom<String> for CategoryType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "income" => Ok(Self::Income),
            "expense" => Ok(Self::Expense),
            other => Err(format!("unknown category type {other:?}")),
        }
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    #[sqlx(rename = "type", try_from = "String")]
    pub kind: CategoryType,
    pub active: bool,
    /// The default "Juros e encargos" category, where Charges are recorded.
    pub is_charges: bool,
    pub created_at: DateTime<Utc>,
}

pub struct CategoryRepository {
    pool: PgPool,
}

impl CategoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The user's categories (active and inactive), optionally narrowed by
    /// type and by active state. Ordered by type, then active first, then name.
    pub async fn list(
        &self,
        user_id: UserId,
        kind: Option<CategoryType>,
        active: Option<bool>,
    ) -> Result<Vec<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(
            "SELECT id, name, type, active, is_charges, created_at FROM categories
             WHERE user_id = $1
               AND ($2::text IS NULL OR type = $2)
               AND ($3::bool IS NULL OR active = $3)
             ORDER BY type DESC, active DESC, name",
        )
        .bind(user_id)
        .bind(kind.map(CategoryType::as_str))
        .bind(active)
        .fetch_all(&self.pool)
        .await
    }
}

/// Gives a new user the default categories. Takes the caller's connection so
/// sign-up can do it in the same transaction that creates the user.
pub async fn seed_default_categories(
    connection: &mut PgConnection,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT seed_default_categories($1)")
        .bind(user_id)
        .execute(connection)
        .await?;
    Ok(())
}
