use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::PgPool;

use crate::{
    ids::{CategoryId, CreditCardId, RecurringTransactionId, UserId},
    ledger::{
        SpendingFacts,
        budget::{CardPurchaseExpense, CashExpense, InvoiceExpense},
        cards::Month,
    },
};

#[derive(sqlx::FromRow)]
struct CashRow {
    category_id: CategoryId,
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
    amount_cents: i64,
}

#[derive(sqlx::FromRow)]
struct RecurringRow {
    id: RecurringTransactionId,
    category_id: CategoryId,
}

/// Reads the expenses with their Categories, as facts for the ledger's
/// budget questions (see `ledger::budget`). Amounts come added up per
/// Category and day (or invoice), so the result stays small; which month each
/// belongs to is the ledger's business.
pub struct SpendingRepository {
    pool: PgPool,
}

impl SpendingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Everything that counts as an expense of a Category: expenses outside
    /// the card, the Category of each recurring expense (whose Occurrences the
    /// ledger derives), card purchases and Installments.
    pub async fn facts(&self, user_id: UserId) -> Result<SpendingFacts, sqlx::Error> {
        let cash_expenses = sqlx::query_as::<_, CashRow>(
            "SELECT category_id, date, sum(amount_cents)::bigint AS amount_cents
             FROM transactions
             WHERE user_id = $1 AND type = 'expense'
             GROUP BY category_id, date",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| CashExpense {
            category_id: row.category_id,
            date: row.date,
            amount_cents: row.amount_cents,
        })
        .collect();

        let recurring_categories: HashMap<RecurringTransactionId, CategoryId> =
            sqlx::query_as::<_, RecurringRow>(
                "SELECT id, category_id FROM recurring_transactions
                 WHERE user_id = $1 AND type = 'expense'",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| (row.id, row.category_id))
            .collect();

        let card_purchases = sqlx::query_as::<_, CardRow>(
            "SELECT credit_card_id, category_id, date, sum(amount_cents)::bigint AS amount_cents
             FROM card_purchases
             WHERE user_id = $1
             GROUP BY credit_card_id, category_id, date",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| CardPurchaseExpense {
            card_id: row.credit_card_id,
            category_id: row.category_id,
            date: row.date,
            amount_cents: row.amount_cents,
        })
        .collect();

        let invoice_expenses: Vec<InvoiceExpense> = sqlx::query_as::<_, InvoiceItemRow>(
            "SELECT p.credit_card_id, p.category_id, i.invoice_month,
                    sum(i.amount_cents)::bigint AS amount_cents
             FROM installments i
             JOIN installment_purchases p ON p.id = i.installment_purchase_id
             WHERE i.user_id = $1
             GROUP BY p.credit_card_id, p.category_id, i.invoice_month",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| InvoiceExpense {
            card_id: row.credit_card_id,
            category_id: row.category_id,
            invoice_month: Month::of(row.invoice_month),
            amount_cents: row.amount_cents,
        })
        .collect();

        // Charges (interest, penalty, IOF) are recorded in an invoice, by its
        // closing month, and count in their Category like an Installment does.
        let mut invoice_expenses = invoice_expenses;
        invoice_expenses.extend(
            sqlx::query_as::<_, InvoiceItemRow>(
                "SELECT credit_card_id, category_id, closing_month AS invoice_month,
                        sum(amount_cents)::bigint AS amount_cents
                 FROM invoice_charges
                 WHERE user_id = $1
                 GROUP BY credit_card_id, category_id, closing_month",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| InvoiceExpense {
                card_id: row.credit_card_id,
                category_id: row.category_id,
                invoice_month: Month::of(row.invoice_month),
                amount_cents: row.amount_cents,
            }),
        );

        Ok(SpendingFacts {
            cash_expenses,
            recurring_categories,
            card_purchases,
            invoice_expenses,
        })
    }
}
