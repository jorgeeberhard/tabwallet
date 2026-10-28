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

    /// The user's category of this type with this name (any letter case),
    /// active or not.
    pub async fn find_by_name(
        &self,
        user_id: UserId,
        kind: CategoryType,
        name: &str,
    ) -> Result<Option<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(
            "SELECT id, name, type, active, is_charges, created_at FROM categories
             WHERE user_id = $1 AND type = $2 AND name = $3::citext",
        )
        .bind(user_id)
        .bind(kind.as_str())
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }

    /// Creates an active category. The database enforces that the name is
    /// unique per user and type, so two simultaneous requests cannot both win.
    pub async fn insert(
        &self,
        user_id: UserId,
        kind: CategoryType,
        name: &str,
    ) -> Result<Category, WriteCategoryError> {
        sqlx::query_as::<_, Category>(
            "INSERT INTO categories (user_id, type, name) VALUES ($1, $2, $3)
             RETURNING id, name, type, active, is_charges, created_at",
        )
        .bind(user_id)
        .bind(kind.as_str())
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(WriteCategoryError::from)
    }

    /// One of the user's categories, or `None` if there is none with this id
    /// (including one that belongs to someone else).
    pub async fn find(
        &self,
        user_id: UserId,
        id: CategoryId,
    ) -> Result<Option<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(
            "SELECT id, name, type, active, is_charges, created_at FROM categories
             WHERE user_id = $1 AND id = $2",
        )
        .bind(user_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Renames one of the user's categories; `None` if it does not exist.
    pub async fn rename(
        &self,
        user_id: UserId,
        id: CategoryId,
        name: &str,
    ) -> Result<Option<Category>, WriteCategoryError> {
        sqlx::query_as::<_, Category>(
            "UPDATE categories SET name = $3 WHERE user_id = $1 AND id = $2
             RETURNING id, name, type, active, is_charges, created_at",
        )
        .bind(user_id)
        .bind(id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(WriteCategoryError::from)
    }

    /// Deactivates or reactivates one of the user's categories; `None` if it
    /// does not exist. Never deletes: transactions keep their category.
    pub async fn set_active(
        &self,
        user_id: UserId,
        id: CategoryId,
        active: bool,
    ) -> Result<Option<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(
            "UPDATE categories SET active = $3 WHERE user_id = $1 AND id = $2
             RETURNING id, name, type, active, is_charges, created_at",
        )
        .bind(user_id)
        .bind(id)
        .bind(active)
        .fetch_optional(&self.pool)
        .await
    }

    /// The "Juros e encargos" rule's single check: whether Charges were
    /// recorded in this category, which then cannot be deactivated.
    ///
    /// There is no Charges table yet (a later ticket adds it), so nothing can
    /// have been recorded and this is always `false`. The ticket that creates
    /// Charges replaces this body with a query on them; the deactivation use
    /// case and its HTTP error (`category_has_charges`) are already in place.
    pub async fn has_charges(
        &self,
        _user_id: UserId,
        _category_id: CategoryId,
    ) -> Result<bool, sqlx::Error> {
        Ok(false)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WriteCategoryError {
    /// Another category of the same type already has the name.
    #[error("category name already used")]
    NameTaken,
    #[error(transparent)]
    Database(sqlx::Error),
}

impl From<sqlx::Error> for WriteCategoryError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(ref db) if db.is_unique_violation() => Self::NameTaken,
            other => Self::Database(other),
        }
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
