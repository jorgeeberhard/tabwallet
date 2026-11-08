use std::{cmp::Reverse, collections::HashMap};

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::{
    ids::{
        CardPurchaseId, CategoryId, ChargeId, CreditCardId, InstallmentPurchaseId,
        InvoicePaymentId, RecurringTransactionId, TransactionId, UserId,
    },
    ledger::{
        Ledger, Occurrence,
        cards::{InvoiceCycle, InvoicePayment, Month, PaymentOrigin},
        projection_horizon,
    },
    repositories::{
        card_purchases::CardPurchaseRepository,
        credit_cards::{CreditCard, CreditCardRepository},
        installment_purchases::InstallmentPurchaseRepository,
        invoice_charges::InvoiceChargeRepository,
        invoice_payments::InvoicePaymentRepository,
        recurring::{RecurringRepository, RecurringTransaction},
        statement::{StatementKey, StatementRepository},
        transactions::{Transaction, TransactionFilter, TransactionRepository, TransactionType},
    },
};

use super::{
    TransactionError,
    card_purchase::CardPurchaseView,
    credit_card::{card_ledger, view_of},
    installment_purchase::{InstallmentPurchaseView, views_of},
    invoice_charge::{ChargeView, view_of as charge_view_of},
};

/// One Occurrence of a recurring Transaction (a Gasto fixo or Receita fixa) in
/// the statement, with the data of the recurring Transaction it came from. It
/// is derived on every read: it is a Projection while its date is after today.
#[derive(Clone, Debug)]
pub struct StatementOccurrence {
    pub recurring_id: RecurringTransactionId,
    /// First day of its month; with `recurring_id` it identifies the
    /// Occurrence.
    pub month: NaiveDate,
    pub kind: TransactionType,
    pub amount_cents: i64,
    pub date: NaiveDate,
    pub description: String,
    pub category_id: CategoryId,
    pub category_name: String,
    pub category_active: bool,
    /// The user edited the amount of this Occurrence only.
    pub edited: bool,
    /// When the recurring Transaction was created; it breaks ties in the
    /// order of the statement.
    pub created_at: DateTime<Utc>,
}

impl StatementOccurrence {
    pub fn new(recurring: &RecurringTransaction, occurrence: Occurrence) -> Self {
        Self {
            recurring_id: recurring.id,
            month: occurrence.month,
            kind: recurring.kind,
            amount_cents: occurrence.amount_cents,
            date: occurrence.date,
            description: recurring.description.clone(),
            category_id: recurring.category_id,
            category_name: recurring.category_name.clone(),
            category_active: recurring.category_active,
            edited: occurrence.edited,
            created_at: recurring.created_at,
        }
    }
}

/// One Invoice Payment in the statement: an Early Payment the user made, or
/// the automatic payment of an invoice's due date (derived on every read). It
/// is not an expense and has no Category; it carries the card and the invoice
/// it pays.
#[derive(Clone, Debug)]
pub struct StatementInvoicePayment {
    pub payment: InvoicePayment,
    pub credit_card_name: String,
    pub credit_card_active: bool,
    pub invoice: InvoiceCycle,
    /// When an Early Payment was recorded; `None` for the automatic payment.
    pub created_at: Option<DateTime<Utc>>,
}

/// One entry of the statement: a Transaction outside the credit card, a card
/// purchase (with its card and invoice), an Occurrence of a recurring
/// Transaction or an Invoice Payment (early or automatic). Each kind has its
/// own data instead of being squeezed into `Transaction`.
#[derive(Clone, Debug)]
pub enum StatementItem {
    Transaction(Transaction),
    CardPurchase(CardPurchaseView),
    InstallmentPurchase(InstallmentPurchaseView),
    /// A Charge (Encargo) recorded in an invoice: an expense that leaves the
    /// balance with the invoice payment.
    Charge(ChargeView),
    Occurrence(StatementOccurrence),
    InvoicePayment(StatementInvoicePayment),
}

impl StatementItem {
    /// The order of the statement, newest first: date, then creation, then id
    /// (for an Occurrence, the id of its recurring Transaction). It is the
    /// order the database uses for Transactions and card purchases, so all can
    /// be merged.
    fn sort_key(&self) -> (NaiveDate, DateTime<Utc>, Uuid) {
        match self {
            Self::Transaction(transaction) => {
                (transaction.date, transaction.created_at, transaction.id.0)
            }
            Self::CardPurchase(view) => (
                view.purchase.date,
                view.purchase.created_at,
                view.purchase.id.0,
            ),
            Self::InstallmentPurchase(view) => (
                view.purchase.date,
                view.purchase.created_at,
                view.purchase.id.0,
            ),
            Self::Charge(view) => (view.charge.date, view.charge.created_at, view.charge.id.0),
            Self::Occurrence(occurrence) => (
                occurrence.date,
                occurrence.created_at,
                occurrence.recurring_id.0,
            ),
            // An automatic payment is derived, so it has no creation time: it
            // goes last in its day, and its card keeps two of them apart.
            Self::InvoicePayment(item) => (
                item.payment.date,
                item.created_at.unwrap_or(DateTime::<Utc>::MIN_UTC),
                match item.payment.origin {
                    PaymentOrigin::Early(id) => id.0,
                    PaymentOrigin::Automatic | PaymentOrigin::Corrected => item.payment.card_id.0,
                },
            ),
        }
    }
}

/// One page of the statement and how many items match in all.
pub struct Statement {
    pub items: Vec<StatementItem>,
    pub total: i64,
}

/// UC08: lists the user's Transactions, card purchases, Invoice Payments and
/// the Occurrences of their recurring Transactions, newest first, optionally
/// narrowed by period, Category, type and credit card.
///
/// Transactions, card purchases and Early Payments are ordered and paged by the
/// database (`StatementRepository`). Occurrences and the automatic Invoice
/// Payments of due dates are computed here from the ledger, never read from
/// the database, and merged in. Without an end date the statement reaches the
/// projection horizon (the end of the month 12 months from now); a later `to`
/// is cut there. A credit card filter leaves the Occurrences out (they never
/// belong to a card) and keeps that card's payments. A Category or type filter
/// leaves the payments out: they have neither.
pub struct ListTransactionsUseCase {
    statement: StatementRepository,
    transactions: TransactionRepository,
    purchases: CardPurchaseRepository,
    installment_purchases: InstallmentPurchaseRepository,
    cards: CreditCardRepository,
    recurring: RecurringRepository,
    payments: InvoicePaymentRepository,
    charges: InvoiceChargeRepository,
}

impl ListTransactionsUseCase {
    // One repository per kind of statement entry, so the list grows with them.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        statement: StatementRepository,
        transactions: TransactionRepository,
        purchases: CardPurchaseRepository,
        installment_purchases: InstallmentPurchaseRepository,
        cards: CreditCardRepository,
        recurring: RecurringRepository,
        payments: InvoicePaymentRepository,
        charges: InvoiceChargeRepository,
    ) -> Self {
        Self {
            statement,
            transactions,
            purchases,
            installment_purchases,
            cards,
            recurring,
            payments,
            charges,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Statement, TransactionError> {
        let mut computed: Vec<StatementItem> = if filter.credit_card_id.is_some() {
            Vec::new()
        } else {
            self.occurrences(user_id, today, filter)
                .await?
                .into_iter()
                .map(StatementItem::Occurrence)
                .collect()
        };
        computed.extend(self.automatic_payments(user_id, today, filter).await?);
        let total = self
            .statement
            .count(user_id, filter)
            .await
            .map_err(TransactionError::internal)?;

        if computed.is_empty() {
            return Ok(Statement {
                items: self.stored(user_id, today, filter, limit, offset).await?,
                total,
            });
        }

        // The page is the `offset..offset + limit` slice of both sorted lists
        // merged, so the first `offset + limit` of each list are enough.
        let computed_count = computed.len() as i64;
        let head = (offset + limit).max(0);
        let mut items = self.stored(user_id, today, filter, head, 0).await?;
        items.extend(computed);
        items.sort_by_key(|item| Reverse(item.sort_key()));
        Ok(Statement {
            total: total + computed_count,
            items: items
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect(),
        })
    }

    /// One page of what is stored (Transactions, card purchases and Early
    /// Payments), in the order of the statement.
    async fn stored(
        &self,
        user_id: UserId,
        today: NaiveDate,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StatementItem>, TransactionError> {
        let keys = self
            .statement
            .page(user_id, filter, limit, offset)
            .await
            .map_err(TransactionError::internal)?;

        let transaction_ids: Vec<TransactionId> = keys
            .iter()
            .filter_map(|key| match key {
                StatementKey::Transaction(id) => Some(*id),
                _ => None,
            })
            .collect();
        let purchase_ids: Vec<CardPurchaseId> = keys
            .iter()
            .filter_map(|key| match key {
                StatementKey::CardPurchase(id) => Some(*id),
                _ => None,
            })
            .collect();

        let installment_purchase_ids: Vec<InstallmentPurchaseId> = keys
            .iter()
            .filter_map(|key| match key {
                StatementKey::InstallmentPurchase(id) => Some(*id),
                _ => None,
            })
            .collect();
        let charge_ids: Vec<ChargeId> = keys
            .iter()
            .filter_map(|key| match key {
                StatementKey::Charge(id) => Some(*id),
                _ => None,
            })
            .collect();
        let payment_ids: Vec<InvoicePaymentId> = keys
            .iter()
            .filter_map(|key| match key {
                StatementKey::InvoicePayment(id) => Some(*id),
                _ => None,
            })
            .collect();

        let mut transactions: HashMap<TransactionId, Transaction> = self
            .transactions
            .find_many(user_id, &transaction_ids)
            .await
            .map_err(TransactionError::internal)?
            .into_iter()
            .map(|transaction| (transaction.id, transaction))
            .collect();
        let mut purchases: HashMap<CardPurchaseId, CardPurchaseView> = HashMap::new();
        let mut installment_purchases: HashMap<InstallmentPurchaseId, InstallmentPurchaseView> =
            HashMap::new();
        let mut payments: HashMap<InvoicePaymentId, StatementInvoicePayment> = HashMap::new();
        if !purchase_ids.is_empty()
            || !installment_purchase_ids.is_empty()
            || !payment_ids.is_empty()
        {
            let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id)
                .await
                .map_err(TransactionError::internal)?;
            for payment in self
                .payments
                .find_many(user_id, &payment_ids)
                .await
                .map_err(TransactionError::internal)?
            {
                let month = Month::of(payment.closing_month);
                if let Some(invoice) = ledger.invoice(payment.credit_card_id, month, today) {
                    payments.insert(
                        payment.id,
                        StatementInvoicePayment {
                            payment: InvoicePayment {
                                card_id: payment.credit_card_id,
                                closing_month: month,
                                date: payment.date,
                                amount_cents: payment.amount_cents,
                                origin: PaymentOrigin::Early(payment.id),
                            },
                            credit_card_name: payment.credit_card_name,
                            credit_card_active: payment.credit_card_active,
                            invoice: invoice.cycle,
                            created_at: Some(payment.created_at),
                        },
                    );
                }
            }
            for purchase in self
                .purchases
                .find_many(user_id, &purchase_ids)
                .await
                .map_err(TransactionError::internal)?
            {
                if let Some(view) = view_of(&ledger, purchase, today) {
                    purchases.insert(view.purchase.id, view);
                }
            }
            let found = self
                .installment_purchases
                .find_many(user_id, &installment_purchase_ids)
                .await
                .map_err(TransactionError::internal)?;
            for view in views_of(&self.installment_purchases, &ledger, user_id, found, today)
                .await
                .map_err(TransactionError::internal)?
            {
                installment_purchases.insert(view.purchase.id, view);
            }
        }
        let mut charges: HashMap<ChargeId, ChargeView> = HashMap::new();
        if !charge_ids.is_empty() {
            let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id)
                .await
                .map_err(TransactionError::internal)?;
            for charge in self
                .charges
                .find_many(user_id, &charge_ids)
                .await
                .map_err(TransactionError::internal)?
            {
                if let Some(view) = charge_view_of(&ledger, charge, today) {
                    charges.insert(view.charge.id, view);
                }
            }
        }

        Ok(keys
            .into_iter()
            .filter_map(|key| match key {
                StatementKey::Transaction(id) => {
                    transactions.remove(&id).map(StatementItem::Transaction)
                }
                StatementKey::CardPurchase(id) => {
                    purchases.remove(&id).map(StatementItem::CardPurchase)
                }
                StatementKey::InstallmentPurchase(id) => installment_purchases
                    .remove(&id)
                    .map(StatementItem::InstallmentPurchase),
                StatementKey::Charge(id) => charges.remove(&id).map(StatementItem::Charge),
                StatementKey::InvoicePayment(id) => {
                    payments.remove(&id).map(StatementItem::InvoicePayment)
                }
            })
            .collect())
    }

    /// The automatic Invoice Payments (invoices' due dates) inside the filter's
    /// period, up to the projection horizon, for the filter's card if it names
    /// one. Those on or before today have already left the Current Balance; the
    /// later ones are projections. A Category or type filter excludes them.
    async fn automatic_payments(
        &self,
        user_id: UserId,
        today: NaiveDate,
        filter: TransactionFilter,
    ) -> Result<Vec<StatementItem>, TransactionError> {
        if filter.category_id.is_some() || filter.kind.is_some() {
            return Ok(Vec::new());
        }
        let (cards, ledger) = card_ledger(&self.cards, &self.purchases, user_id)
            .await
            .map_err(TransactionError::internal)?;
        let from = filter.from.unwrap_or(NaiveDate::MIN);
        let to = filter.to.map_or(projection_horizon(today), |to| {
            to.min(projection_horizon(today))
        });
        let card_named = |id: CreditCardId| cards.iter().find(|card: &&CreditCard| card.id == id);
        Ok(ledger
            .invoice_payments(from, to)
            .into_iter()
            .filter(|payment| payment.origin.is_derived())
            .filter(|payment| {
                filter
                    .credit_card_id
                    .is_none_or(|card_id| card_id == payment.card_id)
            })
            .filter_map(|payment| {
                let card = card_named(payment.card_id)?;
                let invoice = ledger.invoice(payment.card_id, payment.closing_month, today)?;
                Some(StatementItem::InvoicePayment(StatementInvoicePayment {
                    payment,
                    credit_card_name: card.name.clone(),
                    credit_card_active: card.active,
                    invoice: invoice.cycle,
                    created_at: None,
                }))
            })
            .collect())
    }

    /// The Occurrences inside the filter's period (up to the projection
    /// horizon) whose Category and type match.
    async fn occurrences(
        &self,
        user_id: UserId,
        today: NaiveDate,
        filter: TransactionFilter,
    ) -> Result<Vec<StatementOccurrence>, TransactionError> {
        let recurring = self
            .recurring
            .list(user_id)
            .await
            .map_err(TransactionError::internal)?;
        if recurring.is_empty() {
            return Ok(Vec::new());
        }
        let from = filter.from.unwrap_or(NaiveDate::MIN);
        let to = filter.to.map_or(projection_horizon(today), |to| {
            to.min(projection_horizon(today))
        });
        let ledger = Ledger::of_recurring(
            recurring
                .iter()
                .map(|recurring| recurring.definition.clone())
                .collect(),
        );
        Ok(ledger
            .occurrences(from, to)
            .into_iter()
            .filter_map(|occurrence| {
                let source = recurring
                    .iter()
                    .find(|recurring| recurring.id == occurrence.recurring_id)?;
                let kind_matches = filter.kind.is_none_or(|kind| kind == source.kind);
                let category_matches = filter
                    .category_id
                    .is_none_or(|category_id| category_id == source.category_id);
                (kind_matches && category_matches)
                    .then(|| StatementOccurrence::new(source, occurrence))
            })
            .collect())
    }
}
