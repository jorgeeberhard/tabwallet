use chrono::NaiveDate;

use crate::{
    ids::{CreditCardId, UserId},
    ledger::{
        Ledger, LedgerFacts,
        cards::{CardFacts, CardSchedule, Invoice, InvoiceCycle, Month},
    },
    repositories::{
        card_purchases::{CardPurchase, CardPurchaseRepository},
        credit_cards::{CreditCard, CreditCardRepository, WriteCardError},
    },
};

use super::card_purchase::CardPurchaseView;

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
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl CreditCardError {
    pub(super) fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

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

/// The ledger over all of a user's cards and their purchases, with the cards.
/// Every invoice question starts here.
pub(super) async fn card_ledger(
    cards: &CreditCardRepository,
    purchases: &CardPurchaseRepository,
    user_id: UserId,
) -> Result<(Vec<CreditCard>, Ledger), CreditCardError> {
    let list = cards
        .list(user_id)
        .await
        .map_err(CreditCardError::internal)?;
    let mut schedules = cards
        .schedules(user_id)
        .await
        .map_err(CreditCardError::internal)?;
    let mut spending = purchases
        .spending(user_id)
        .await
        .map_err(CreditCardError::internal)?;
    let facts = list
        .iter()
        .filter_map(|card| {
            Some(CardFacts {
                id: card.id,
                schedule: schedules.remove(&card.id)?,
                spending: spending.remove(&card.id).unwrap_or_default(),
            })
        })
        .collect();
    Ok((
        list,
        Ledger::new(LedgerFacts {
            cards: facts,
            ..LedgerFacts::default()
        }),
    ))
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
}

impl UpdateCreditCardUseCase {
    pub fn new(cards: CreditCardRepository) -> Self {
        Self { cards }
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
        if let Some(name) = changes.name {
            card = self
                .cards
                .rename(user_id, id, &name)
                .await?
                .ok_or(CreditCardError::NotFound)?;
        }
        let latest = *schedule.latest();
        let closing_day = changes.closing_day.unwrap_or(latest.closing_day);
        let due_day = changes.due_day.unwrap_or(latest.due_day);
        if (closing_day, due_day) != (latest.closing_day, latest.due_day) {
            let from = schedule.change_takes_effect_from(closing_day, today);
            self.cards
                .change_days(id, from, closing_day, due_day)
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
}

/// The invoice of a card closing in one month, with its purchases.
pub struct GetInvoiceUseCase {
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
}

impl GetInvoiceUseCase {
    pub fn new(cards: CreditCardRepository, purchases: CardPurchaseRepository) -> Self {
        Self { cards, purchases }
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
        Ok(InvoiceDetail { invoice, purchases })
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
