use chrono::NaiveDate;

use crate::{
    ids::{CreditCardId, UserId},
    ledger::cards::{
        CardChange, CorrectionFact, CorrectionState, Invoice, InvoicePayment, InvoiceState, Month,
        PaymentOrigin,
    },
    repositories::{
        card_purchases::CardPurchaseRepository,
        credit_cards::CreditCardRepository,
        invoice_payments::{EarlyPayment, InvoicePaymentRepository},
    },
};

use super::{CreditCardError, credit_card::card_ledger};

/// Why paying an invoice early failed. Unaware of HTTP.
#[derive(Debug, thiserror::Error)]
pub enum InvoicePaymentError {
    /// There is no such card, or it belongs to another user.
    #[error("credit card not found")]
    CardNotFound,
    /// The card has no such invoice.
    #[error("invoice not found")]
    InvoiceNotFound,
    /// The invoice is Paid: its due date has come and nothing can be paid.
    #[error("the invoice is paid")]
    InvoicePaid,
    /// The corrected payment would leave the next invoice with a total below
    /// what was already paid on it early.
    #[error("the change would leave an invoice overpaid")]
    WouldBeOverpaid,
    /// The amount is more than the invoice still has to pay.
    #[error("the amount is more than the invoice's remaining amount")]
    AmountExceedsRemaining,
    /// The due date has not come, so there is no payment to correct yet.
    #[error("the invoice is not due yet")]
    NotDue,
    /// Early Payments covered the whole invoice: the due date debits nothing.
    #[error("the invoice has no due-date payment to correct")]
    NothingToPay,
    /// The invoice that received the Carried Balance is Paid, which locks the
    /// payment.
    #[error("the payment is locked")]
    Locked,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl InvoicePaymentError {
    fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<CreditCardError> for InvoicePaymentError {
    fn from(error: CreditCardError) -> Self {
        match error {
            CreditCardError::NotFound => Self::CardNotFound,
            CreditCardError::InvoiceNotFound => Self::InvoiceNotFound,
            other => Self::internal(other),
        }
    }
}

/// The Early Payment that was made and the invoice as it stands after it.
pub struct EarlyPaymentReceipt {
    pub payment: EarlyPayment,
    pub invoice: Invoice,
}

/// The corrected due-date payment and the invoice as it stands after it.
pub struct CorrectionReceipt {
    pub payment: InvoicePayment,
    pub invoice: Invoice,
}

/// Corrects the payment of an invoice to the amount really paid at the
/// bank. The invoice must be Paid (its due date has come) and the one after
/// it, which receives what is left unpaid as a Carried Balance, must not be
/// Paid yet. The amount is zero up to what the Early Payments left unpaid;
/// the difference moves to the next invoice. Correcting again replaces the
/// earlier correction. Nothing but the corrected amount is stored.
pub struct CorrectInvoicePaymentUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
    payments: InvoicePaymentRepository,
}

impl CorrectInvoicePaymentUseCase {
    pub fn new(
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
        payments: InvoicePaymentRepository,
    ) -> Self {
        Self {
            cards,
            purchases,
            payments,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        closing_month: Month,
        amount_cents: i64,
        today: NaiveDate,
    ) -> Result<CorrectionReceipt, InvoicePaymentError> {
        let (cards, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        if !cards.iter().any(|card| card.id == card_id) {
            return Err(InvoicePaymentError::CardNotFound);
        }
        let invoice = ledger
            .invoice(card_id, closing_month, today)
            .ok_or(InvoicePaymentError::InvoiceNotFound)?;
        match invoice.payment_correction.state {
            CorrectionState::NotDue => return Err(InvoicePaymentError::NotDue),
            CorrectionState::Locked => return Err(InvoicePaymentError::Locked),
            CorrectionState::NothingToPay => return Err(InvoicePaymentError::NothingToPay),
            CorrectionState::Editable => {}
        }
        if amount_cents > invoice.payment_correction.max_cents {
            return Err(InvoicePaymentError::AmountExceedsRemaining);
        }
        let change = CardChange {
            correction: Some(CorrectionFact {
                closing_month,
                amount_cents,
            }),
            ..CardChange::default()
        };
        if ledger.overpaid_by(card_id, &change).is_some() {
            return Err(InvoicePaymentError::WouldBeOverpaid);
        }
        self.payments
            .set_correction(user_id, card_id, closing_month, amount_cents)
            .await
            .map_err(InvoicePaymentError::internal)?;
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let invoice = ledger
            .invoice(card_id, closing_month, today)
            .ok_or(InvoicePaymentError::InvoiceNotFound)?;
        Ok(CorrectionReceipt {
            payment: InvoicePayment {
                card_id,
                closing_month,
                date: invoice.cycle.due_date,
                amount_cents,
                origin: PaymentOrigin::Corrected,
            },
            invoice,
        })
    }
}

/// Pays an Open or Closed invoice before its due date, in full or in
/// part, for at most what it still has to pay. The payment is dated today (an
/// Early Payment is never in the future) and leaves the Current Balance at
/// once; it is not an expense. Purchases added later only raise the remaining
/// amount again.
pub struct PayInvoiceUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
    payments: InvoicePaymentRepository,
}

impl PayInvoiceUseCase {
    pub fn new(
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
        payments: InvoicePaymentRepository,
    ) -> Self {
        Self {
            cards,
            purchases,
            payments,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        closing_month: Month,
        amount_cents: i64,
        today: NaiveDate,
    ) -> Result<EarlyPaymentReceipt, InvoicePaymentError> {
        let (cards, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        if !cards.iter().any(|card| card.id == card_id) {
            return Err(InvoicePaymentError::CardNotFound);
        }
        let invoice = ledger
            .invoice(card_id, closing_month, today)
            .ok_or(InvoicePaymentError::InvoiceNotFound)?;
        if invoice.cycle.state == InvoiceState::Paid {
            return Err(InvoicePaymentError::InvoicePaid);
        }
        if amount_cents > invoice.remaining_cents {
            return Err(InvoicePaymentError::AmountExceedsRemaining);
        }
        let payment = self
            .payments
            .insert_within_total(
                user_id,
                card_id,
                closing_month,
                amount_cents,
                today,
                invoice.total_cents,
            )
            .await
            .map_err(InvoicePaymentError::internal)?
            .ok_or(InvoicePaymentError::AmountExceedsRemaining)?;
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let invoice = ledger
            .invoice(card_id, closing_month, today)
            .ok_or(InvoicePaymentError::InvoiceNotFound)?;
        Ok(EarlyPaymentReceipt { payment, invoice })
    }
}
