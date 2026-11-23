use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::{
    ids::{CategoryId, ChargeId, CreditCardId, UserId},
    ledger::cards::Month,
};

/// A Charge (Encargo) as stored: interest, penalty or IOF the bank charged on
/// an invoice, recorded by the user as an expense of the "Juros e encargos"
/// Category. It belongs to the invoice the user linked it to, by card and
/// closing month; its `date` is only when the bank charged it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct InvoiceCharge {
    pub id: ChargeId,
    pub credit_card_id: CreditCardId,
    pub credit_card_name: String,
    pub credit_card_active: bool,
    /// The first day of the month the invoice closes in.
    pub closing_month: NaiveDate,
    /// Always greater than zero.
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
    pub category_name: String,
    pub category_active: bool,
    pub created_at: DateTime<Utc>,
}

/// What a new Charge is made of. The caller has already checked the card (the
/// user's), the invoice (it exists and is not paid) and the Category (the
/// user's active "Juros e encargos").
pub struct NewCharge {
    pub credit_card_id: CreditCardId,
    pub closing_month: Month,
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
}

/// What an edit may change; `None` leaves the field as it is. The card, the
/// invoice and the Category never change.
#[derive(Default)]
pub struct ChargeChanges {
    pub amount_cents: Option<i64>,
    pub date: Option<NaiveDate>,
    pub description: Option<String>,
}

/// The columns of [`InvoiceCharge`]; `ch` is the charges table (or a CTE with
/// its columns), the card is joined as `cc` and the Category as `c`.
macro_rules! columns {
    () => {
        "ch.id, ch.credit_card_id, cc.name AS credit_card_name, cc.active AS credit_card_active,
         ch.closing_month, ch.amount_cents, ch.date, ch.description, ch.category_id,
         c.name AS category_name, c.active AS category_active, ch.created_at"
    };
}

macro_rules! joins {
    () => {
        " JOIN credit_cards cc ON cc.id = ch.credit_card_id
          JOIN categories c ON c.id = ch.category_id"
    };
}

pub struct InvoiceChargeRepository {
    pool: PgPool,
}

impl InvoiceChargeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        user_id: UserId,
        new: NewCharge,
    ) -> Result<InvoiceCharge, sqlx::Error> {
        sqlx::query_as::<_, InvoiceCharge>(concat!(
            "WITH ch AS (
                 INSERT INTO invoice_charges
                     (user_id, credit_card_id, closing_month, amount_cents, date, description,
                      category_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 RETURNING *
             )
             SELECT ",
            columns!(),
            " FROM ch",
            joins!()
        ))
        .bind(user_id)
        .bind(new.credit_card_id)
        .bind(new.closing_month.first_day())
        .bind(new.amount_cents)
        .bind(new.date)
        .bind(new.description)
        .bind(new.category_id)
        .fetch_one(&self.pool)
        .await
    }

    /// One of the user's Charges, or `None` if there is none with this id
    /// (including one that belongs to someone else).
    pub async fn find(
        &self,
        user_id: UserId,
        id: ChargeId,
    ) -> Result<Option<InvoiceCharge>, sqlx::Error> {
        sqlx::query_as::<_, InvoiceCharge>(concat!(
            "SELECT ",
            columns!(),
            " FROM invoice_charges ch",
            joins!(),
            " WHERE ch.user_id = $1 AND ch.id = $2"
        ))
        .bind(user_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// The Charges with these ids (of the user), in no particular order.
    pub async fn find_many(
        &self,
        user_id: UserId,
        ids: &[ChargeId],
    ) -> Result<Vec<InvoiceCharge>, sqlx::Error> {
        let ids: Vec<uuid::Uuid> = ids.iter().map(|id| id.0).collect();
        sqlx::query_as::<_, InvoiceCharge>(concat!(
            "SELECT ",
            columns!(),
            " FROM invoice_charges ch",
            joins!(),
            " WHERE ch.user_id = $1 AND ch.id = ANY($2)"
        ))
        .bind(user_id)
        .bind(ids)
        .fetch_all(&self.pool)
        .await
    }

    /// The Charges of one invoice, newest first (the order of the statement).
    pub async fn list_for_invoice(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        closing_month: Month,
    ) -> Result<Vec<InvoiceCharge>, sqlx::Error> {
        sqlx::query_as::<_, InvoiceCharge>(concat!(
            "SELECT ",
            columns!(),
            " FROM invoice_charges ch",
            joins!(),
            " WHERE ch.user_id = $1 AND ch.credit_card_id = $2 AND ch.closing_month = $3
              ORDER BY ch.date DESC, ch.created_at DESC, ch.id DESC"
        ))
        .bind(user_id)
        .bind(card_id)
        .bind(closing_month.first_day())
        .fetch_all(&self.pool)
        .await
    }

    /// Applies the changes to one of the user's Charges; `None` if it does not
    /// exist.
    pub async fn update(
        &self,
        user_id: UserId,
        id: ChargeId,
        changes: ChargeChanges,
    ) -> Result<Option<InvoiceCharge>, sqlx::Error> {
        sqlx::query_as::<_, InvoiceCharge>(concat!(
            "WITH ch AS (
                 UPDATE invoice_charges SET
                     amount_cents = COALESCE($3, amount_cents),
                     date = COALESCE($4, date),
                     description = COALESCE($5, description),
                     updated_at = now()
                 WHERE user_id = $1 AND id = $2
                 RETURNING *
             )
             SELECT ",
            columns!(),
            " FROM ch",
            joins!()
        ))
        .bind(user_id)
        .bind(id)
        .bind(changes.amount_cents)
        .bind(changes.date)
        .bind(changes.description)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes one of the user's Charges; `false` if it does not exist.
    pub async fn delete(&self, user_id: UserId, id: ChargeId) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM invoice_charges WHERE user_id = $1 AND id = $2")
            .bind(user_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
