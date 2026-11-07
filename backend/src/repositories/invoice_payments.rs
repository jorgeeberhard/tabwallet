use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::{
    ids::{CreditCardId, InvoicePaymentId, UserId},
    ledger::cards::Month,
};

/// An Early Payment of an invoice, as stored: money that left the balance on
/// `date` to pay part (or all) of the invoice closing in `closing_month`. The
/// payment that settles an invoice on its Due Day is never stored.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct EarlyPayment {
    pub id: InvoicePaymentId,
    pub credit_card_id: CreditCardId,
    pub credit_card_name: String,
    pub credit_card_active: bool,
    /// The first day of the month the invoice closes in.
    pub closing_month: NaiveDate,
    /// Always greater than zero.
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

/// The columns of [`EarlyPayment`]; `p` is the payments table (or a CTE with
/// its columns) and the card is joined as `cc`.
macro_rules! columns {
    () => {
        "p.id, p.credit_card_id, cc.name AS credit_card_name, cc.active AS credit_card_active,
         p.closing_month, p.amount_cents, p.date, p.created_at"
    };
}

pub struct InvoicePaymentRepository {
    pool: PgPool,
}

impl InvoicePaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Records an Early Payment unless the invoice's Early Payments, with this
    /// one, would pass `invoice_total_cents` (`None` then). Payments to the
    /// same card are serialized by a lock, so two requests at once cannot both
    /// pass the check.
    pub async fn insert_within_total(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        closing_month: Month,
        amount_cents: i64,
        date: NaiveDate,
        invoice_total_cents: i64,
    ) -> Result<Option<EarlyPayment>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(card_id.0.to_string())
            .execute(&mut *tx)
            .await?;
        let paid: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(amount_cents), 0)::bigint FROM invoice_payments
             WHERE credit_card_id = $1 AND closing_month = $2",
        )
        .bind(card_id)
        .bind(closing_month.first_day())
        .fetch_one(&mut *tx)
        .await?;
        if paid + amount_cents > invoice_total_cents {
            return Ok(None);
        }
        let payment = sqlx::query_as::<_, EarlyPayment>(concat!(
            "WITH p AS (
                 INSERT INTO invoice_payments
                     (user_id, credit_card_id, closing_month, amount_cents, date)
                 VALUES ($1, $2, $3, $4, $5)
                 RETURNING *
             )
             SELECT ",
            columns!(),
            " FROM p JOIN credit_cards cc ON cc.id = p.credit_card_id"
        ))
        .bind(user_id)
        .bind(card_id)
        .bind(closing_month.first_day())
        .bind(amount_cents)
        .bind(date)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(payment))
    }

    /// Stores the amount the user really paid of an invoice at its due date,
    /// replacing an earlier correction of the same invoice. The caller has
    /// checked that the invoice is the user's and can be corrected.
    pub async fn set_correction(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        closing_month: Month,
        amount_cents: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO invoice_payment_corrections
                 (credit_card_id, closing_month, user_id, amount_cents)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (credit_card_id, closing_month)
             DO UPDATE SET amount_cents = EXCLUDED.amount_cents, updated_at = now()",
        )
        .bind(card_id)
        .bind(closing_month.first_day())
        .bind(user_id)
        .bind(amount_cents)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The payments with these ids (of the user), in no particular order.
    pub async fn find_many(
        &self,
        user_id: UserId,
        ids: &[InvoicePaymentId],
    ) -> Result<Vec<EarlyPayment>, sqlx::Error> {
        let ids: Vec<uuid::Uuid> = ids.iter().map(|id| id.0).collect();
        sqlx::query_as::<_, EarlyPayment>(concat!(
            "SELECT ",
            columns!(),
            " FROM invoice_payments p JOIN credit_cards cc ON cc.id = p.credit_card_id
              WHERE p.user_id = $1 AND p.id = ANY($2)"
        ))
        .bind(user_id)
        .bind(ids)
        .fetch_all(&self.pool)
        .await
    }
}
