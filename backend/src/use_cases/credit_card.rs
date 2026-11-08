use chrono::NaiveDate;

use crate::{
    ids::{CreditCardId, UserId},
    ledger::{
        Ledger,
        cards::{
            CardChange, CardFacts, CardSchedule, Invoice, InvoiceCycle, InvoicePayment, Month,
        },
    },
    repositories::{
        card_purchases::{CardPurchase, CardPurchaseRepository},
        credit_cards::{CreditCard, CreditCardRepository, WriteCardError},
        installment_purchases::InstallmentPurchaseRepository,
        invoice_charges::InvoiceChargeRepository,
    },
};

use super::{
    card_purchase::CardPurchaseView,
    installment_purchase::{InvoiceInstallment, invoice_installments, plan_reallocation},
    invoice_charge::{ChargeView, view_of as charge_view_of},
};

/// Why a credit-card use case failed. Unaware of HTTP: the handler layer
/// decides the status code.
#[derive(Debug, thiserror::Error)]
pub enum CreditCardError {
    /// There is no such card, or it belongs to another user.
    #[error("credit card not found")]
    NotFound,
    /// There is no such invoice for the card.
    #[error("invoice not found")]
    InvoiceNotFound,
    /// Another card of the user has this name.
    #[error("credit card name already used")]
    NameTaken,
    /// An invoice still has an amount to pay, so the card stays active.
    #[error("the credit card still has an invoice with an amount to pay")]
    HasRemainingInvoice,
    /// The change would leave an invoice with a total below what was already
    /// paid on it early.
    #[error("the change would leave an invoice overpaid")]
    WouldBeOverpaid,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

internal_error!(CreditCardError);

impl From<WriteCardError> for CreditCardError {
    fn from(error: WriteCardError) -> Self {
        match error {
            WriteCardError::NameTaken => Self::NameTaken,
            WriteCardError::Database(cause) => Self::internal(cause),
        }
    }
}

/// A card with the closing and due days it has from now on.
#[derive(Clone, Debug)]
pub struct CreditCardView {
    pub card: CreditCard,
    pub closing_day: u32,
    pub due_day: u32,
}

impl CreditCardView {
    fn new(card: CreditCard, schedule: &CardSchedule) -> Self {
        let latest = schedule.latest();
        Self {
            card,
            closing_day: latest.closing_day,
            due_day: latest.due_day,
        }
    }
}

/// What the ledger needs to know about all of a user's cards: their days,
/// purchases and Early Payments.
pub(super) async fn card_facts(
    cards: &CreditCardRepository,
    purchases: &CardPurchaseRepository,
    user_id: UserId,
) -> Result<(Vec<CreditCard>, Vec<CardFacts>), sqlx::Error> {
    // Seven independent reads: run together, so a request waits for one round
    // trip to the database instead of seven.
    let (
        list,
        mut schedules,
        mut spending,
        mut installments,
        mut payments,
        mut charges,
        mut corrections,
    ) = tokio::try_join!(
        cards.list(user_id),
        cards.schedules(user_id),
        purchases.spending(user_id),
        cards.installment_charges(user_id),
        cards.payments(user_id),
        cards.charges(user_id),
        cards.corrections(user_id),
    )?;
    let facts = list
        .iter()
        .filter_map(|card| {
            Some(CardFacts {
                id: card.id,
                schedule: schedules.remove(&card.id)?,
                spending: spending.remove(&card.id).unwrap_or_default(),
                installments: installments.remove(&card.id).unwrap_or_default(),
                payments: payments.remove(&card.id).unwrap_or_default(),
                charges: charges.remove(&card.id).unwrap_or_default(),
                corrections: corrections.remove(&card.id).unwrap_or_default(),
            })
        })
        .collect();
    Ok((list, facts))
}

/// The ledger over all of a user's cards and their purchases, with the cards.
/// Every invoice question starts here.
pub(super) async fn card_ledger(
    cards: &CreditCardRepository,
    purchases: &CardPurchaseRepository,
    user_id: UserId,
) -> Result<(Vec<CreditCard>, Ledger), CreditCardError> {
    let (list, facts) = card_facts(cards, purchases, user_id)
        .await
        .map_err(CreditCardError::internal)?;
    Ok((list, Ledger::of_cards(facts)))
}

/// UC16: registers a card with its Closing Day and Due Day (1 to 31).
pub struct CreateCreditCardUseCase {
    cards: CreditCardRepository,
}

impl CreateCreditCardUseCase {
    pub fn new(cards: CreditCardRepository) -> Self {
        Self { cards }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        name: &str,
        closing_day: u32,
        due_day: u32,
    ) -> Result<CreditCardView, CreditCardError> {
        let card = self
            .cards
            .insert(user_id, name, closing_day, due_day)
            .await?;
        Ok(CreditCardView {
            card,
            closing_day,
            due_day,
        })
    }
}

/// UC16: lists the user's cards, active ones first.
pub struct ListCreditCardsUseCase {
    cards: CreditCardRepository,
}

impl ListCreditCardsUseCase {
    pub fn new(cards: CreditCardRepository) -> Self {
        Self { cards }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<Vec<CreditCardView>, CreditCardError> {
        let list = self
            .cards
            .list(user_id)
            .await
            .map_err(CreditCardError::internal)?;
        let mut schedules = self
            .cards
            .schedules(user_id)
            .await
            .map_err(CreditCardError::internal)?;
        Ok(list
            .into_iter()
            .filter_map(|card| {
                let schedule = schedules.remove(&card.id)?;
                Some(CreditCardView::new(card, &schedule))
            })
            .collect())
    }
}

/// What an edit of a card may change; `None` leaves the field as it is.
#[derive(Default)]
pub struct CreditCardChanges {
    pub name: Option<String>,
    pub closing_day: Option<u32>,
    pub due_day: Option<u32>,
}

/// UC16: renames a card and/or changes its days. New days only affect the
/// invoices still open (see [`CardSchedule::change_takes_effect_from`]):
/// closed and paid invoices keep the days they had.
pub struct UpdateCreditCardUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
    installment_purchases: InstallmentPurchaseRepository,
}

impl UpdateCreditCardUseCase {
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
        id: CreditCardId,
        changes: CreditCardChanges,
        today: NaiveDate,
    ) -> Result<CreditCardView, CreditCardError> {
        let mut card = self
            .cards
            .find(user_id, id)
            .await
            .map_err(CreditCardError::internal)?
            .ok_or(CreditCardError::NotFound)?;
        let mut schedule = self
            .cards
            .schedules(user_id)
            .await
            .map_err(CreditCardError::internal)?
            .remove(&id)
            .ok_or(CreditCardError::NotFound)?;
        let latest = *schedule.latest();
        let closing_day = changes.closing_day.unwrap_or(latest.closing_day);
        let due_day = changes.due_day.unwrap_or(latest.due_day);
        let new_days = if (closing_day, due_day) != (latest.closing_day, latest.due_day) {
            let from = schedule.change_takes_effect_from(closing_day, today);
            let new_schedule = schedule.with_days_from(from, closing_day, due_day);
            // Installments are stored with their invoice, so the new days
            // must move them the way they move single purchases. Nothing is
            // stored until we know no invoice would be left with less than
            // was paid on it.
            let reallocation = plan_reallocation(
                &self.installment_purchases,
                user_id,
                &CardFacts {
                    id,
                    schedule: new_schedule.clone(),
                    spending: Vec::new(),
                    installments: Vec::new(),
                    payments: Vec::new(),
                    charges: Vec::new(),
                    corrections: Vec::new(),
                },
                today,
            )
            .await
            .map_err(CreditCardError::internal)?;
            let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
            let change = CardChange {
                schedule: Some(new_schedule),
                installments_removed: reallocation.removed.clone(),
                installments_added: reallocation.added.clone(),
                ..CardChange::default()
            };
            if ledger.overpaid_by(id, &change).is_some() {
                return Err(CreditCardError::WouldBeOverpaid);
            }
            Some((from, closing_day, due_day, reallocation))
        } else {
            None
        };
        if let Some(name) = changes.name {
            card = self
                .cards
                .rename(user_id, id, &name)
                .await?
                .ok_or(CreditCardError::NotFound)?;
        }
        if let Some((from, closing_day, due_day, reallocation)) = new_days {
            if !self
                .cards
                .change_days(user_id, id, from, closing_day, due_day)
                .await
                .map_err(CreditCardError::internal)?
            {
                return Err(CreditCardError::NotFound);
            }
            reallocation
                .apply(&self.installment_purchases, user_id)
                .await
                .map_err(CreditCardError::internal)?;
            schedule = self
                .cards
                .schedules(user_id)
                .await
                .map_err(CreditCardError::internal)?
                .remove(&id)
                .ok_or(CreditCardError::NotFound)?;
        }
        Ok(CreditCardView::new(card, &schedule))
    }
}

/// UC16: deactivates a card, which then takes no new purchases but keeps its
/// history. Only possible when no invoice has an amount left to pay.
pub struct DeactivateCreditCardUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
}

impl DeactivateCreditCardUseCase {
    pub fn new(cards: CreditCardRepository, purchases: CardPurchaseRepository) -> Self {
        Self { cards, purchases }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: CreditCardId,
        today: NaiveDate,
    ) -> Result<CreditCardView, CreditCardError> {
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        if !self
            .cards
            .find(user_id, id)
            .await
            .map_err(CreditCardError::internal)?
            .is_some()
        {
            return Err(CreditCardError::NotFound);
        }
        if ledger.has_remaining_invoice(id, today) {
            return Err(CreditCardError::HasRemainingInvoice);
        }
        set_active(&self.cards, user_id, id, false).await
    }
}

/// UC16: makes a deactivated card available for new purchases again.
pub struct ReactivateCreditCardUseCase {
    cards: CreditCardRepository,
}

impl ReactivateCreditCardUseCase {
    pub fn new(cards: CreditCardRepository) -> Self {
        Self { cards }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: CreditCardId,
    ) -> Result<CreditCardView, CreditCardError> {
        set_active(&self.cards, user_id, id, true).await
    }
}

async fn set_active(
    cards: &CreditCardRepository,
    user_id: UserId,
    id: CreditCardId,
    active: bool,
) -> Result<CreditCardView, CreditCardError> {
    let card = cards
        .set_active(user_id, id, active)
        .await
        .map_err(CreditCardError::internal)?
        .ok_or(CreditCardError::NotFound)?;
    let schedule = cards
        .schedules(user_id)
        .await
        .map_err(CreditCardError::internal)?
        .remove(&id)
        .ok_or(CreditCardError::NotFound)?;
    Ok(CreditCardView::new(card, &schedule))
}

/// UC16/UC15: the invoices of a card, oldest first, with state, dates, total
/// and what remains to pay.
pub struct ListInvoicesUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
}

impl ListInvoicesUseCase {
    pub fn new(cards: CreditCardRepository, purchases: CardPurchaseRepository) -> Self {
        Self { cards, purchases }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        today: NaiveDate,
    ) -> Result<Vec<Invoice>, CreditCardError> {
        let (cards, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        if !cards.iter().any(|card| card.id == card_id) {
            return Err(CreditCardError::NotFound);
        }
        Ok(ledger.invoices(card_id, today))
    }
}

/// One invoice with the purchases that make it up.
pub struct InvoiceDetail {
    pub invoice: Invoice,
    /// Newest first.
    pub purchases: Vec<CardPurchaseView>,
    /// The Installments in the invoice, newest purchase first.
    pub installments: Vec<InvoiceInstallment>,
    /// The Charges recorded in the invoice, newest first.
    pub charges: Vec<ChargeView>,
    /// The payments of the invoice, oldest first: the Early Payments and, once
    /// there is something unpaid, the due-date one (automatic or corrected).
    pub payments: Vec<InvoicePayment>,
}

/// The invoice of a card closing in one month, with its purchases and
/// Installments.
pub struct GetInvoiceUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
    installment_purchases: InstallmentPurchaseRepository,
    charges: InvoiceChargeRepository,
}

impl GetInvoiceUseCase {
    pub fn new(
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
        installment_purchases: InstallmentPurchaseRepository,
        charges: InvoiceChargeRepository,
    ) -> Self {
        Self {
            cards,
            purchases,
            installment_purchases,
            charges,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        closing_month: Month,
        today: NaiveDate,
    ) -> Result<InvoiceDetail, CreditCardError> {
        let (cards, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        if !cards.iter().any(|card| card.id == card_id) {
            return Err(CreditCardError::NotFound);
        }
        let invoice = ledger
            .invoice(card_id, closing_month, today)
            .ok_or(CreditCardError::InvoiceNotFound)?;
        let purchases = self
            .purchases
            .list_for_card(user_id, card_id)
            .await
            .map_err(CreditCardError::internal)?
            .into_iter()
            .filter_map(|purchase| view_of(&ledger, purchase, today))
            .filter(|view| view.invoice.closing_month == closing_month)
            .collect();
        let installments = invoice_installments(
            &self.installment_purchases,
            &ledger,
            user_id,
            card_id,
            closing_month,
            today,
        )
        .await
        .map_err(CreditCardError::internal)?;
        let charges = self
            .charges
            .list_for_invoice(user_id, card_id, closing_month)
            .await
            .map_err(CreditCardError::internal)?
            .into_iter()
            .filter_map(|charge| charge_view_of(&ledger, charge, today))
            .collect();
        let payments = ledger.payments_of_invoice(card_id, closing_month);
        Ok(InvoiceDetail {
            invoice,
            purchases,
            installments,
            charges,
            payments,
        })
    }
}

/// A purchase with the invoice it enters, as the ledger sees it.
pub(super) fn view_of(
    ledger: &Ledger,
    purchase: CardPurchase,
    today: NaiveDate,
) -> Option<CardPurchaseView> {
    let invoice: InvoiceCycle =
        ledger.invoice_for_purchase(purchase.credit_card_id, purchase.date, today)?;
    Some(CardPurchaseView { purchase, invoice })
}
