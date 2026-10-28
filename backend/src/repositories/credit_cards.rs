use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::{
    ids::{CreditCardId, UserId},
    ledger::cards::{CardSchedule, Month, ScheduleVersion},
};

/// A credit card. Its closing and due days are not here: they have a history
/// (see [`CardSchedule`]) and are read with
/// [`CreditCardRepository::schedules`].
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct CreditCard {
    pub id: CreditCardId,
    pub name: String,
    /// A deactivated card takes no new purchases but keeps its history.
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

/// The columns of [`CreditCard`]. A macro because `sqlx` only takes literal
/// SQL and `concat!` needs a literal.
macro_rules! columns {
    () => {
        "id, name, active, created_at"
    };
}

#[derive(Debug, thiserror::Error)]
pub enum WriteCardError {
    /// Another card of the user has this name.
    #[error("credit card name already used")]
    NameTaken,
    #[error(transparent)]
    Database(sqlx::Error),
}

impl From<sqlx::Error> for WriteCardError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(ref db) if db.is_unique_violation() => Self::NameTaken,
            other => Self::Database(other),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ScheduleRow {
    credit_card_id: CreditCardId,
    effective_from: NaiveDate,
    closing_day: i16,
    due_day: i16,
}

impl ScheduleRow {
    fn version(&self) -> ScheduleVersion {
        ScheduleVersion {
            effective_from: Month::of(self.effective_from),
            closing_day: self.closing_day as u32,
            due_day: self.due_day as u32,
        }
    }
}

pub struct CreditCardRepository {
    pool: PgPool,
}

impl CreditCardRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates an active card whose days apply to every invoice (the first
    /// schedule version starts at the beginning of time).
    pub async fn insert(
        &self,
        user_id: UserId,
        name: &str,
        closing_day: u32,
        due_day: u32,
    ) -> Result<CreditCard, WriteCardError> {
        let mut tx = self.pool.begin().await?;
        let card = sqlx::query_as::<_, CreditCard>(concat!(
            "INSERT INTO credit_cards (user_id, name) VALUES ($1, $2) RETURNING ",
            columns!()
        ))
        .bind(user_id)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO credit_card_schedules (credit_card_id, effective_from, closing_day, due_day)
             VALUES ($1, '0001-01-01', $2, $3)",
        )
        .bind(card.id)
        .bind(closing_day as i16)
        .bind(due_day as i16)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(card)
    }

    /// The user's cards, active first, then by name.
    pub async fn list(&self, user_id: UserId) -> Result<Vec<CreditCard>, sqlx::Error> {
        sqlx::query_as::<_, CreditCard>(concat!(
            "SELECT ",
            columns!(),
            " FROM credit_cards WHERE user_id = $1
             ORDER BY active DESC, lower(name), id"
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// One of the user's cards, or `None` if there is none with this id
    /// (including one that belongs to someone else).
    pub async fn find(
        &self,
        user_id: UserId,
        id: CreditCardId,
    ) -> Result<Option<CreditCard>, sqlx::Error> {
        sqlx::query_as::<_, CreditCard>(concat!(
            "SELECT ",
            columns!(),
            " FROM credit_cards WHERE user_id = $1 AND id = $2"
        ))
        .bind(user_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn rename(
        &self,
        user_id: UserId,
        id: CreditCardId,
        name: &str,
    ) -> Result<Option<CreditCard>, WriteCardError> {
        sqlx::query_as::<_, CreditCard>(concat!(
            "UPDATE credit_cards SET name = $3, updated_at = now()
             WHERE user_id = $1 AND id = $2 RETURNING ",
            columns!()
        ))
        .bind(user_id)
        .bind(id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(WriteCardError::from)
    }

    /// Deactivates or reactivates a card; never deletes.
    pub async fn set_active(
        &self,
        user_id: UserId,
        id: CreditCardId,
        active: bool,
    ) -> Result<Option<CreditCard>, sqlx::Error> {
        sqlx::query_as::<_, CreditCard>(concat!(
            "UPDATE credit_cards SET active = $3, updated_at = now()
             WHERE user_id = $1 AND id = $2 RETURNING ",
            columns!()
        ))
        .bind(user_id)
        .bind(id)
        .bind(active)
        .fetch_optional(&self.pool)
        .await
    }

    /// The days history of every card of the user.
    pub async fn schedules(
        &self,
        user_id: UserId,
    ) -> Result<HashMap<CreditCardId, CardSchedule>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ScheduleRow>(
            "SELECT s.credit_card_id, s.effective_from, s.closing_day, s.due_day
             FROM credit_card_schedules s JOIN credit_cards c ON c.id = s.credit_card_id
             WHERE c.user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        let mut grouped: HashMap<CreditCardId, Vec<ScheduleVersion>> = HashMap::new();
        for row in &rows {
            grouped
                .entry(row.credit_card_id)
                .or_default()
                .push(row.version());
        }
        Ok(grouped
            .into_iter()
            .filter_map(|(id, versions)| CardSchedule::new(versions).map(|s| (id, s)))
            .collect())
    }

    /// Starts new closing and due days at the invoice closing in `from`,
    /// replacing any version that already starts there or later.
    pub async fn change_days(
        &self,
        id: CreditCardId,
        from: Month,
        closing_day: u32,
        due_day: u32,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "DELETE FROM credit_card_schedules WHERE credit_card_id = $1 AND effective_from >= $2",
        )
        .bind(id)
        .bind(from.first_day())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO credit_card_schedules (credit_card_id, effective_from, closing_day, due_day)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(from.first_day())
        .bind(closing_day as i16)
        .bind(due_day as i16)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }
}
