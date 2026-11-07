use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::{
    ids::{CategoryId, CreditCardId, InstallmentId, InstallmentPurchaseId, UserId},
    ledger::cards::Month,
};

/// A card purchase split in Installments: an expense linked to a card. The
/// Installments are stored apart (see [`Installment`]); which invoice each
/// one is in is stored with it, not worked out from this record.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct InstallmentPurchase {
    pub id: InstallmentPurchaseId,
    pub credit_card_id: CreditCardId,
    pub credit_card_name: String,
    pub credit_card_active: bool,
    /// What the user informed, already with interest if any. Greater than zero.
    pub total_cents: i64,
    /// How many Installments it was launched in (2 or more). It does not
    /// change when the remaining Installments are cancelled.
    pub installment_count: i32,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
    pub category_name: String,
    pub category_active: bool,
    /// The remaining Installments were cancelled after some had been paid.
    pub cancelled: bool,
    pub created_at: DateTime<Utc>,
}

/// One Installment: its own record, with its own amount and the invoice it is
/// in, so that a later operation can move a single one to another invoice.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Installment {
    pub id: InstallmentId,
    pub installment_purchase_id: InstallmentPurchaseId,
    /// 1 to the purchase's `installment_count`.
    pub number: i32,
    /// Greater than zero.
    pub amount_cents: i64,
    /// The first day of the closing month of the invoice it is in.
    pub invoice_month: NaiveDate,
}

impl Installment {
    pub fn invoice_month(&self) -> Month {
        Month::of(self.invoice_month)
    }
}

/// What a new purchase is made of. The caller has already checked the card
/// (the user's, active) and the Category (the user's, active, an expense one).
pub struct NewInstallmentPurchase {
    pub credit_card_id: CreditCardId,
    pub total_cents: i64,
    pub installment_count: u32,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
}

/// The amount and invoice the ledger gave to one Installment.
#[derive(Clone, Copy, Debug)]
pub struct Allocation {
    pub number: u32,
    pub amount_cents: i64,
    pub invoice_month: Month,
}

/// What an edit may change; `None` leaves the field as it is. The card never
/// changes.
#[derive(Default)]
pub struct InstallmentPurchaseChanges {
    pub total_cents: Option<i64>,
    pub installment_count: Option<u32>,
    pub date: Option<NaiveDate>,
    pub description: Option<String>,
    pub category_id: Option<CategoryId>,
}

/// The columns of [`InstallmentPurchase`]; `p` is the purchases table (or a
/// CTE with its columns), the card is joined as `cc` and the Category as `c`.
macro_rules! columns {
    () => {
        "p.id, p.credit_card_id, cc.name AS credit_card_name, cc.active AS credit_card_active,
         p.total_cents, p.installment_count, p.date, p.description, p.category_id,
         c.name AS category_name, c.active AS category_active,
         (p.cancelled_at IS NOT NULL) AS cancelled, p.created_at"
    };
}

macro_rules! joins {
    () => {
        " JOIN credit_cards cc ON cc.id = p.credit_card_id
          JOIN categories c ON c.id = p.category_id"
    };
}

pub struct InstallmentPurchaseRepository {
    pool: PgPool,
}

impl InstallmentPurchaseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Stores the purchase and its Installments together.
    pub async fn insert(
        &self,
        user_id: UserId,
        new: NewInstallmentPurchase,
        allocations: &[Allocation],
    ) -> Result<InstallmentPurchase, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let id: InstallmentPurchaseId = sqlx::query_scalar(
            "INSERT INTO installment_purchases
                 (user_id, credit_card_id, total_cents, installment_count, date,
                  description, category_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id",
        )
        .bind(user_id)
        .bind(new.credit_card_id)
        .bind(new.total_cents)
        .bind(new.installment_count as i32)
        .bind(new.date)
        .bind(new.description)
        .bind(new.category_id)
        .fetch_one(&mut *tx)
        .await?;
        upsert_installments(&mut tx, user_id, id, allocations).await?;
        tx.commit().await?;
        Ok(self
            .find(user_id, id)
            .await?
            .expect("the purchase was just stored"))
    }

    /// One of the user's purchases, or `None` if there is none with this id
    /// (including one that belongs to someone else).
    pub async fn find(
        &self,
        user_id: UserId,
        id: InstallmentPurchaseId,
    ) -> Result<Option<InstallmentPurchase>, sqlx::Error> {
        sqlx::query_as::<_, InstallmentPurchase>(concat!(
            "SELECT ",
            columns!(),
            " FROM installment_purchases p",
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
        ids: &[InstallmentPurchaseId],
    ) -> Result<Vec<InstallmentPurchase>, sqlx::Error> {
        let ids: Vec<uuid::Uuid> = ids.iter().map(|id| id.0).collect();
        sqlx::query_as::<_, InstallmentPurchase>(concat!(
            "SELECT ",
            columns!(),
            " FROM installment_purchases p",
            joins!(),
            " WHERE p.user_id = $1 AND p.id = ANY($2)"
        ))
        .bind(user_id)
        .bind(ids)
        .fetch_all(&self.pool)
        .await
    }

    /// Every purchase of one card, newest first.
    pub async fn list_for_card(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
    ) -> Result<Vec<InstallmentPurchase>, sqlx::Error> {
        sqlx::query_as::<_, InstallmentPurchase>(concat!(
            "SELECT ",
            columns!(),
            " FROM installment_purchases p",
            joins!(),
            " WHERE p.user_id = $1 AND p.credit_card_id = $2
              ORDER BY p.date DESC, p.created_at DESC, p.id DESC"
        ))
        .bind(user_id)
        .bind(card_id)
        .fetch_all(&self.pool)
        .await
    }

    /// The Installments of these purchases, by purchase and number.
    pub async fn installments(
        &self,
        user_id: UserId,
        purchase_ids: &[InstallmentPurchaseId],
    ) -> Result<Vec<Installment>, sqlx::Error> {
        let ids: Vec<uuid::Uuid> = purchase_ids.iter().map(|id| id.0).collect();
        sqlx::query_as::<_, Installment>(
            "SELECT id, installment_purchase_id, number, amount_cents, invoice_month
             FROM installments
             WHERE user_id = $1 AND installment_purchase_id = ANY($2)
             ORDER BY installment_purchase_id, number",
        )
        .bind(user_id)
        .bind(ids)
        .fetch_all(&self.pool)
        .await
    }

    /// Applies the changes to one of the user's purchases and, when
    /// `allocations` is given, replaces its Installments by them (keeping the
    /// record of each number that still exists); `None` if the purchase does
    /// not exist.
    pub async fn update(
        &self,
        user_id: UserId,
        id: InstallmentPurchaseId,
        changes: InstallmentPurchaseChanges,
        allocations: Option<&[Allocation]>,
    ) -> Result<Option<InstallmentPurchase>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE installment_purchases SET
                 total_cents = COALESCE($3, total_cents),
                 installment_count = COALESCE($4, installment_count),
                 date = COALESCE($5, date),
                 description = COALESCE($6, description),
                 category_id = COALESCE($7, category_id),
                 updated_at = now()
             WHERE user_id = $1 AND id = $2",
        )
        .bind(user_id)
        .bind(id)
        .bind(changes.total_cents)
        .bind(changes.installment_count.map(|count| count as i32))
        .bind(changes.date)
        .bind(changes.description)
        .bind(changes.category_id)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() == 0 {
            return Ok(None);
        }
        if let Some(allocations) = allocations {
            sqlx::query(
                "DELETE FROM installments WHERE installment_purchase_id = $1 AND number > $2",
            )
            .bind(id)
            .bind(allocations.len() as i32)
            .execute(&mut *tx)
            .await?;
            upsert_installments(&mut tx, user_id, id, allocations).await?;
        }
        tx.commit().await?;
        self.find(user_id, id).await
    }

    /// Removes the Installments with these ids and marks the purchase as
    /// cancelled; `None` if the purchase does not exist.
    pub async fn cancel_installments(
        &self,
        user_id: UserId,
        id: InstallmentPurchaseId,
        installment_ids: &[InstallmentId],
    ) -> Result<Option<InstallmentPurchase>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let marked = sqlx::query(
            "UPDATE installment_purchases SET cancelled_at = now(), updated_at = now()
             WHERE user_id = $1 AND id = $2",
        )
        .bind(user_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if marked.rows_affected() == 0 {
            return Ok(None);
        }
        let ids: Vec<uuid::Uuid> = installment_ids.iter().map(|id| id.0).collect();
        sqlx::query("DELETE FROM installments WHERE installment_purchase_id = $1 AND id = ANY($2)")
            .bind(id)
            .bind(ids)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.find(user_id, id).await
    }

    /// Deletes one of the user's purchases with all its Installments; `false`
    /// if it does not exist.
    pub async fn delete(
        &self,
        user_id: UserId,
        id: InstallmentPurchaseId,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM installment_purchases WHERE user_id = $1 AND id = $2")
                .bind(user_id)
                .bind(id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// Writes one row per allocation: the Installment of that number is created,
/// or kept (same id) with its new amount and invoice.
async fn upsert_installments(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: UserId,
    purchase_id: InstallmentPurchaseId,
    allocations: &[Allocation],
) -> Result<(), sqlx::Error> {
    for allocation in allocations {
        sqlx::query(
            "INSERT INTO installments
                 (user_id, installment_purchase_id, number, amount_cents, invoice_month)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (installment_purchase_id, number)
             DO UPDATE SET amount_cents = EXCLUDED.amount_cents,
                           invoice_month = EXCLUDED.invoice_month",
        )
        .bind(user_id)
        .bind(purchase_id)
        .bind(allocation.number as i32)
        .bind(allocation.amount_cents)
        .bind(allocation.invoice_month.first_day())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
