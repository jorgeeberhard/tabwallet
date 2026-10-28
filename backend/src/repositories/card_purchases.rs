use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::{
    ids::{CardPurchaseId, CategoryId, CreditCardId, UserId},
    ledger::cards::CardSpend,
};

/// A purchase made with a credit card: an expense linked to a card. It enters
/// an invoice by its date and the card's days, which the ledger works out;
/// nothing about the invoice is stored with it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct CardPurchase {
    pub id: CardPurchaseId,
    pub credit_card_id: CreditCardId,
    pub credit_card_name: String,
    pub credit_card_active: bool,
    /// Always greater than zero.
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
    pub category_name: String,
    pub category_active: bool,
    pub created_at: DateTime<Utc>,
}

/// What a new purchase is made of. The caller has already checked the card
/// (the user's, active) and the Category (the user's, active, an expense one).
pub struct NewCardPurchase {
    pub credit_card_id: CreditCardId,
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
}

/// What an edit may change; `None` leaves the field as it is. The card never
/// changes.
#[derive(Default)]
pub struct CardPurchaseChanges {
    pub amount_cents: Option<i64>,
    pub date: Option<NaiveDate>,
    pub description: Option<String>,
    pub category_id: Option<CategoryId>,
}

/// The columns of [`CardPurchase`]; `p` is the card purchases table (or a CTE
/// with its columns), the card is joined as `cc` and the Category as `c`.
macro_rules! columns {
    () => {
        "p.id, p.credit_card_id, cc.name AS credit_card_name, cc.active AS credit_card_active,
         p.amount_cents, p.date, p.description, p.category_id,
         c.name AS category_name, c.active AS category_active, p.created_at"
    };
}

macro_rules! joins {
    () => {
        " JOIN credit_cards cc ON cc.id = p.credit_card_id
          JOIN categories c ON c.id = p.category_id"
    };
}

#[derive(sqlx::FromRow)]
struct SpendRow {
    credit_card_id: CreditCardId,
    date: NaiveDate,
    amount_cents: i64,
}

pub struct CardPurchaseRepository {
    pool: PgPool,
}

impl CardPurchaseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        user_id: UserId,
        new: NewCardPurchase,
    ) -> Result<CardPurchase, sqlx::Error> {
        sqlx::query_as::<_, CardPurchase>(concat!(
            "WITH p AS (
                 INSERT INTO card_purchases
                     (user_id, credit_card_id, amount_cents, date, description, category_id)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING *
             )
             SELECT ",
            columns!(),
            " FROM p",
            joins!()
        ))
        .bind(user_id)
        .bind(new.credit_card_id)
        .bind(new.amount_cents)
        .bind(new.date)
        .bind(new.description)
        .bind(new.category_id)
        .fetch_one(&self.pool)
        .await
    }

    /// One of the user's purchases, or `None` if there is none with this id
    /// (including one that belongs to someone else).
    pub async fn find(
        &self,
        user_id: UserId,
        id: CardPurchaseId,
    ) -> Result<Option<CardPurchase>, sqlx::Error> {
        sqlx::query_as::<_, CardPurchase>(concat!(
            "SELECT ",
            columns!(),
            " FROM card_purchases p",
            joins!(),
            " WHERE p.user_id = $1 AND p.id = $2"
        ))
        .bind(user_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// The purchases with these ids (of the user), in no particular order.
    pub async fn find_many(
        &self,
        user_id: UserId,
        ids: &[CardPurchaseId],
    ) -> Result<Vec<CardPurchase>, sqlx::Error> {
        let ids: Vec<uuid::Uuid> = ids.iter().map(|id| id.0).collect();
        sqlx::query_as::<_, CardPurchase>(concat!(
            "SELECT ",
            columns!(),
            " FROM card_purchases p",
            joins!(),
            " WHERE p.user_id = $1 AND p.id = ANY($2)"
        ))
        .bind(user_id)
        .bind(ids)
        .fetch_all(&self.pool)
        .await
    }

    /// Every purchase of one card, newest first (the order of the statement).
    pub async fn list_for_card(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
    ) -> Result<Vec<CardPurchase>, sqlx::Error> {
        sqlx::query_as::<_, CardPurchase>(concat!(
            "SELECT ",
            columns!(),
            " FROM card_purchases p",
            joins!(),
            " WHERE p.user_id = $1 AND p.credit_card_id = $2
              ORDER BY p.date DESC, p.created_at DESC, p.id DESC"
        ))
        .bind(user_id)
        .bind(card_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Applies the changes to one of the user's purchases; `None` if it does
    /// not exist.
    pub async fn update(
        &self,
        user_id: UserId,
        id: CardPurchaseId,
        changes: CardPurchaseChanges,
    ) -> Result<Option<CardPurchase>, sqlx::Error> {
        sqlx::query_as::<_, CardPurchase>(concat!(
            "WITH p AS (
                 UPDATE card_purchases SET
                     amount_cents = COALESCE($3, amount_cents),
                     date = COALESCE($4, date),
                     description = COALESCE($5, description),
                     category_id = COALESCE($6, category_id),
                     updated_at = now()
                 WHERE user_id = $1 AND id = $2
                 RETURNING *
             )
             SELECT ",
            columns!(),
            " FROM p",
            joins!()
        ))
        .bind(user_id)
        .bind(id)
        .bind(changes.amount_cents)
        .bind(changes.date)
        .bind(changes.description)
        .bind(changes.category_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes one of the user's purchases; `false` if it does not exist.
    pub async fn delete(&self, user_id: UserId, id: CardPurchaseId) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM card_purchases WHERE user_id = $1 AND id = $2")
            .bind(user_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// The user's purchases as facts for the ledger: the amounts added up per
    /// card and day. Which invoice they enter is the ledger's business.
    pub async fn spending(
        &self,
        user_id: UserId,
    ) -> Result<HashMap<CreditCardId, Vec<CardSpend>>, sqlx::Error> {
        let rows = sqlx::query_as::<_, SpendRow>(
            "SELECT credit_card_id, date, sum(amount_cents)::bigint AS amount_cents
             FROM card_purchases
             WHERE user_id = $1
             GROUP BY credit_card_id, date",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        let mut grouped: HashMap<CreditCardId, Vec<CardSpend>> = HashMap::new();
        for row in rows {
            grouped
                .entry(row.credit_card_id)
                .or_default()
                .push(CardSpend {
                    date: row.date,
                    amount_cents: row.amount_cents,
                });
        }
        Ok(grouped)
    }
}
