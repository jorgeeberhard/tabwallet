use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::PgPool;

use crate::{
    ids::{CategoryId, UserId},
    ledger::{Budget, budget::LimitVersion, cards::Month},
};

/// The Budget of one Category with the name and state of the Category, for
/// listing. The limit history is what the ledger works from.
#[derive(Clone, Debug)]
pub struct StoredBudget {
    pub category_name: String,
    pub budget: Budget,
}

#[derive(sqlx::FromRow)]
struct VersionRow {
    category_id: CategoryId,
    category_name: String,
    category_active: bool,
    effective_month: NaiveDate,
    limit_cents: Option<i64>,
}

pub struct BudgetRepository {
    pool: PgPool,
}

impl BudgetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Every Budget the user ever defined (also those removed and those of
    /// deactivated Categories), with the history of its limit, ordered by
    /// Category name. Which limit is in force in a month is the ledger's
    /// business.
    pub async fn list(&self, user_id: UserId) -> Result<Vec<StoredBudget>, sqlx::Error> {
        let rows = sqlx::query_as::<_, VersionRow>(
            "SELECT b.category_id, c.name::text AS category_name, c.active AS category_active,
                    b.effective_month, b.limit_cents
             FROM budget_limits b
             JOIN categories c ON c.id = b.category_id
             WHERE b.user_id = $1
             ORDER BY c.name, b.category_id, b.effective_month",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut order: Vec<CategoryId> = Vec::new();
        let mut by_category: HashMap<CategoryId, StoredBudget> = HashMap::new();
        for row in rows {
            let stored = by_category.entry(row.category_id).or_insert_with(|| {
                order.push(row.category_id);
                StoredBudget {
                    category_name: row.category_name.clone(),
                    budget: Budget {
                        category_id: row.category_id,
                        category_active: row.category_active,
                        versions: Vec::new(),
                    },
                }
            });
            stored.budget.versions.push(LimitVersion {
                effective_from: Month::of(row.effective_month),
                limit_cents: row.limit_cents,
            });
        }
        Ok(order
            .into_iter()
            .filter_map(|id| by_category.remove(&id))
            .collect())
    }

    /// Starts `limit_cents` as the limit of the Category from `from` on,
    /// replacing the version that already starts in that month, if any. The
    /// caller has already checked that the Category is the user's, an active
    /// expense one, and that the limit is greater than zero.
    pub async fn set_limit(
        &self,
        user_id: UserId,
        category_id: CategoryId,
        from: Month,
        limit_cents: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO budget_limits (user_id, category_id, effective_month, limit_cents)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (category_id, effective_month)
             DO UPDATE SET limit_cents = EXCLUDED.limit_cents, updated_at = now()",
        )
        .bind(user_id)
        .bind(category_id)
        .bind(from.first_day())
        .bind(limit_cents)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Ends the Budget of the Category from `from` on, keeping the limit that
    /// earlier months had. With no earlier limit there is nothing to keep, so
    /// the Budget just disappears.
    pub async fn end_from(
        &self,
        user_id: UserId,
        category_id: CategoryId,
        from: Month,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "DELETE FROM budget_limits
             WHERE user_id = $1 AND category_id = $2 AND effective_month >= $3",
        )
        .bind(user_id)
        .bind(category_id)
        .bind(from.first_day())
        .execute(&mut *tx)
        .await?;
        let has_earlier: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM budget_limits
                 WHERE user_id = $1 AND category_id = $2 AND effective_month < $3
             )",
        )
        .bind(user_id)
        .bind(category_id)
        .bind(from.first_day())
        .fetch_one(&mut *tx)
        .await?;
        if has_earlier {
            sqlx::query(
                "INSERT INTO budget_limits (user_id, category_id, effective_month, limit_cents)
                 VALUES ($1, $2, $3, NULL)",
            )
            .bind(user_id)
            .bind(category_id)
            .bind(from.first_day())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
    }
}
