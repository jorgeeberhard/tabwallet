use chrono::NaiveDate;

use crate::{
    ids::{CreditCardId, InstallmentId, UserId},
    ledger::{
        anticipation::{AnticipationError, InstallmentToAnticipate},
        cards::{Invoice, InvoiceCycle},
    },
    repositories::{
        card_purchases::CardPurchaseRepository,
        credit_cards::CreditCardRepository,
        installment_purchases::{AnticipationRow, InstallmentPurchaseRepository},
    },
};

use super::{
    credit_card::card_ledger,
    installment_purchase::{InstallmentPurchaseView, InvoiceInstallment, views_of},
};

/// Why an Installment Anticipation use case failed. Unaware of HTTP.
#[derive(Debug, thiserror::Error)]
pub enum InstallmentAnticipationError {
    /// There is no such card, or it belongs to another user.
    #[error("credit card not found")]
    CardNotFound,
    /// The user did not confirm. The operation cannot be undone, so it is
    /// never done without an explicit confirmation.
    #[error("the anticipation was not confirmed")]
    NotConfirmed,
    #[error("no installment was chosen")]
    NothingChosen,
    /// Some chosen Installment does not exist, belongs to another card or to
    /// another user.
    #[error("installment not found")]
    InstallmentNotFound,
    /// Some chosen Installment is not in an invoice later than the open one
    /// (it is in the open, a closed or a paid invoice, or was anticipated).
    #[error("some installments cannot be anticipated")]
    NotAnticipable,
    /// The Discount is not positive or would leave an Installment without a
    /// cent.
    #[error("the discount is invalid")]
    DiscountTooLarge,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl InstallmentAnticipationError {
    fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<super::CreditCardError> for InstallmentAnticipationError {
    fn from(error: super::CreditCardError) -> Self {
        match error {
            super::CreditCardError::NotFound => Self::CardNotFound,
            other => Self::internal(other),
        }
    }
}

impl From<super::InstallmentPurchaseError> for InstallmentAnticipationError {
    fn from(error: super::InstallmentPurchaseError) -> Self {
        Self::internal(error)
    }
}

/// The Installments of purchases as lines of the invoices they are in.
fn items_of(views: Vec<InstallmentPurchaseView>) -> Vec<InvoiceInstallment> {
    views
        .into_iter()
        .flat_map(|view| {
            let has_paid = view.has_paid_installments();
            let has_anticipated = view.has_anticipated_installments();
            let purchase = view.purchase;
            view.installments
                .into_iter()
                .map(move |installment| InvoiceInstallment {
                    purchase: purchase.clone(),
                    installment,
                    purchase_has_paid_installments: has_paid,
                    purchase_has_anticipated_installments: has_anticipated,
                })
        })
        .collect()
}

/// Every Installment of a card, with its purchase.
async fn card_installments(
    repository: &InstallmentPurchaseRepository,
    ledger: &crate::ledger::Ledger,
    user_id: UserId,
    card_id: CreditCardId,
    today: NaiveDate,
) -> Result<Vec<InvoiceInstallment>, InstallmentAnticipationError> {
    let purchases = repository
        .list_for_card(user_id, card_id)
        .await
        .map_err(InstallmentAnticipationError::internal)?;
    Ok(items_of(
        views_of(repository, ledger, user_id, purchases, today).await?,
    ))
}

/// The Installments that can be anticipated and the invoice that would
/// receive them.
pub struct AnticipableInstallments {
    /// The open invoice.
    pub open_invoice: InvoiceCycle,
    /// Earliest invoice first, then by purchase date and number.
    pub installments: Vec<InvoiceInstallment>,
}

/// The Installments of a card that sit in invoices after the open one: the
/// ones UC19 lets the user bring to the open invoice.
pub struct ListAnticipableInstallmentsUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
    installment_purchases: InstallmentPurchaseRepository,
}

impl ListAnticipableInstallmentsUseCase {
    pub fn new(
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
        installment_purchases: InstallmentPurchaseRepository,
    ) -> Self {
        Self {
            cards,
            purchases,
            installment_purchases,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        today: NaiveDate,
    ) -> Result<AnticipableInstallments, InstallmentAnticipationError> {
        let (cards, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        if !cards.iter().any(|card| card.id == card_id) {
            return Err(InstallmentAnticipationError::CardNotFound);
        }
        let open_invoice = ledger
            .open_invoice_cycle(card_id, today)
            .ok_or(InstallmentAnticipationError::CardNotFound)?;
        let mut installments: Vec<InvoiceInstallment> = card_installments(
            &self.installment_purchases,
            &ledger,
            user_id,
            card_id,
            today,
        )
        .await?
        .into_iter()
        .filter(|item| item.installment.invoice.closing_month > open_invoice.closing_month)
        .collect();
        installments.sort_by_key(|item| {
            (
                item.installment.invoice.closing_month,
                item.purchase.date,
                item.purchase.created_at,
                item.installment.installment.number,
            )
        });
        Ok(AnticipableInstallments {
            open_invoice,
            installments,
        })
    }
}

/// What an Installment Anticipation did.
pub struct AnticipationReceipt {
    /// The open invoice after receiving the Installments.
    pub invoice: Invoice,
    /// Cents before the Discount.
    pub original_total_cents: i64,
    pub discount_cents: i64,
    /// Cents the Installments add up to in the open invoice.
    pub total_cents: i64,
    /// The anticipated Installments as they are now, earliest origin first.
    pub installments: Vec<InvoiceInstallment>,
}

/// UC19: brings Installments of invoices after the open one into the open
/// invoice of the same card, with an optional Discount split in proportion
/// between them. It is irreversible: nothing is done without `confirmed`, and
/// there is no use case that undoes it. The anticipated Installments count in
/// the open invoice's Reference Month, and the Discount lowers each one and so
/// the expense in the Category of its purchase.
pub struct AnticipateInstallmentsUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
    installment_purchases: InstallmentPurchaseRepository,
}

impl AnticipateInstallmentsUseCase {
    pub fn new(
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
        installment_purchases: InstallmentPurchaseRepository,
    ) -> Self {
        Self {
            cards,
            purchases,
            installment_purchases,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        installment_ids: Vec<InstallmentId>,
        discount_cents: i64,
        confirmed: bool,
        today: NaiveDate,
    ) -> Result<AnticipationReceipt, InstallmentAnticipationError> {
        if !confirmed {
            return Err(InstallmentAnticipationError::NotConfirmed);
        }
        let (cards, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        if !cards.iter().any(|card| card.id == card_id) {
            return Err(InstallmentAnticipationError::CardNotFound);
        }
        // Each chosen id once, in the order given.
        let mut ids = installment_ids;
        let mut seen = std::collections::HashSet::new();
        ids.retain(|id| seen.insert(*id));
        if ids.is_empty() {
            return Err(InstallmentAnticipationError::NothingChosen);
        }

        let all = card_installments(
            &self.installment_purchases,
            &ledger,
            user_id,
            card_id,
            today,
        )
        .await?;
        let mut chosen: Vec<&InvoiceInstallment> = Vec::with_capacity(ids.len());
        for id in &ids {
            chosen.push(
                all.iter()
                    .find(|item| item.installment.installment.id == *id)
                    .ok_or(InstallmentAnticipationError::InstallmentNotFound)?,
            );
        }
        // A tie in the split goes to the earliest purchase, then the lowest
        // number (the ledger puts the earliest invoice first).
        chosen.sort_by_key(|item| {
            (
                item.purchase.date,
                item.purchase.created_at,
                item.installment.installment.number,
            )
        });
        let to_anticipate: Vec<InstallmentToAnticipate> = chosen
            .iter()
            .map(|item| InstallmentToAnticipate {
                id: item.installment.installment.id,
                invoice_month: item.installment.installment.invoice_month(),
                amount_cents: item.installment.installment.amount_cents,
            })
            .collect();
        let plan = ledger
            .plan_anticipation(card_id, &to_anticipate, discount_cents, today)
            .map_err(|error| match error {
                AnticipationError::UnknownCard => InstallmentAnticipationError::CardNotFound,
                AnticipationError::NothingChosen => InstallmentAnticipationError::NothingChosen,
                AnticipationError::NotAnticipable(_) => {
                    InstallmentAnticipationError::NotAnticipable
                }
                AnticipationError::DiscountTooLarge => {
                    InstallmentAnticipationError::DiscountTooLarge
                }
            })?;

        let rows: Vec<AnticipationRow> = plan
            .installments
            .iter()
            .map(|share| AnticipationRow {
                id: share.id,
                from_month: share.from_month,
                original_cents: share.original_cents,
                discount_cents: share.discount_cents,
            })
            .collect();
        let stored = self
            .installment_purchases
            .anticipate(user_id, plan.invoice.closing_month, today, &rows)
            .await
            .map_err(InstallmentAnticipationError::internal)?;
        if !stored {
            return Err(InstallmentAnticipationError::NotAnticipable);
        }

        // Read everything again: the invoice and the Installments changed.
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let invoice = ledger
            .invoice(card_id, plan.invoice.closing_month, today)
            .ok_or(InstallmentAnticipationError::CardNotFound)?;
        let mut after = card_installments(
            &self.installment_purchases,
            &ledger,
            user_id,
            card_id,
            today,
        )
        .await?;
        let installments = plan
            .installments
            .iter()
            .filter_map(|share| {
                let index = after
                    .iter()
                    .position(|item| item.installment.installment.id == share.id)?;
                Some(after.swap_remove(index))
            })
            .collect();
        Ok(AnticipationReceipt {
            invoice,
            original_total_cents: plan.original_total_cents(),
            discount_cents: plan.discount_cents(),
            total_cents: plan.total_cents(),
            installments,
        })
    }
}
