use chrono::NaiveDate;
use sqlx::PgPool;

use super::transactions::TransactionType;
use crate::{
    ids::{CategoryId, CreditCardId, UserId},
    ledger::{
        FlowKind,
        cards::Month,
        reports::{ReportCardPurchase, ReportCashFlow, ReportInvoiceItem},
    },
};

#[derive(sqlx::FromRow)]
struct CashRow {
    category_id: CategoryId,
    #[sqlx(rename = "type", try_from = "String")]
    kind: TransactionType,
    date: NaiveDate,
    amount_cents: i64,
}

#[derive(sqlx::FromRow)]
struct CardRow {
    credit_card_id: CreditCardId,
    category_id: CategoryId,
    date: NaiveDate,
    amount_cents: i64,
}

#[derive(sqlx::FromRow)]
struct InvoiceItemRow {
    credit_card_id: CreditCardId,
    category_id: CategoryId,
    invoice_month: NaiveDate,
    date: NaiveDate,
    amount_cents: i64,
}

/// Reads the incomes and expenses with the Category of each, as facts for the
/// ledger's reports (see `ledger::reports`). Amounts come added up per
/// Category and day (or invoice), so the result stays small; which month each
/// belongs to is the ledger's business.
pub struct ReportRepository {
    pool: PgPool,
}

impl ReportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Incomes and expenses outside the credit card.
    pub async fn cash_flows(&self, user_id: UserId) -> Result<Vec<ReportCashFlow>, sqlx::Error> {
        let rows = sqlx::query_as::<_, CashRow>(
            "SELECT category_id, type, date, sum(amount_cents)::bigint AS amount_cents
             FROM transactions
             WHERE user_id = $1
             GROUP BY category_id, type, date",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ReportCashFlow {
                category_id: row.category_id,
                kind: match row.kind {
                    TransactionType::Income => FlowKind::Income,
                    TransactionType::Expense => FlowKind::Expense,
                },
                date: row.date,
                amount_cents: row.amount_cents,
            })
            .collect())
    }

    /// Single card purchases.
    pub async fn card_purchases(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ReportCardPurchase>, sqlx::Error> {
        let rows = sqlx::query_as::<_, CardRow>(
            "SELECT credit_card_id, category_id, date, sum(amount_cents)::bigint AS amount_cents
             FROM card_purchases
             WHERE user_id = $1
             GROUP BY credit_card_id, category_id, date",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ReportCardPurchase {
                card_id: row.credit_card_id,
                category_id: row.category_id,
                date: row.date,
                amount_cents: row.amount_cents,
            })
            .collect())
    }

    /// Installments (with the amount they have now, less any Discount) and
    /// Charges: the expenses that are in an invoice by the invoice itself.
    pub async fn invoice_items(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ReportInvoiceItem>, sqlx::Error> {
        let rows = sqlx::query_as::<_, InvoiceItemRow>(
            "SELECT p.credit_card_id, p.category_id, i.invoice_month, p.date,
                    sum(i.amount_cents)::bigint AS amount_cents
             FROM installments i
             JOIN installment_purchases p ON p.id = i.installment_purchase_id
             WHERE i.user_id = $1
             GROUP BY p.credit_card_id, p.category_id, i.invoice_month, p.date
             UNION ALL
             SELECT credit_card_id, category_id, closing_month AS invoice_month, date,
                    sum(amount_cents)::bigint AS amount_cents
             FROM invoice_charges
             WHERE user_id = $1
             GROUP BY credit_card_id, category_id, closing_month, date",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ReportInvoiceItem {
                card_id: row.credit_card_id,
                category_id: row.category_id,
                invoice_month: Month::of(row.invoice_month),
                date: row.date,
                amount_cents: row.amount_cents,
            })
            .collect())
    }
}
