use chrono::NaiveDate;

use crate::{
    ids::{CardPurchaseId, UserId},
    ledger::cards::{InvoiceCycle, InvoiceState},
    repositories::{
        card_purchases::{
            CardPurchase, CardPurchaseChanges, CardPurchaseRepository, NewCardPurchase,
        },
        categories::CategoryRepository,
        credit_cards::CreditCardRepository,
        transactions::TransactionType,
    },
};

use super::{
    CategoryProblem, TransactionError,
    credit_card::{card_ledger, view_of},
    transaction_error::ensure_usable_category,
};

/// Why a card cannot take a new purchase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardProblem {
    /// There is no such card, or it belongs to another user.
    NotFound,
    /// The card was deactivated, so it takes no new purchases.
    Inactive,
}

/// Why a card-purchase use case failed. Unaware of HTTP.
#[derive(Debug, thiserror::Error)]
pub enum CardPurchaseError {
    /// There is no such purchase, or it belongs to another user.
    #[error("card purchase not found")]
    NotFound,
    #[error("the category cannot be used: {0:?}")]
    InvalidCategory(CategoryProblem),
    #[error("the card cannot be used: {0:?}")]
    InvalidCard(CardProblem),
    /// The purchase is in a paid invoice, which can no longer be changed.
    #[error("the purchase is in a paid invoice")]
    InvoicePaid,
    /// The date would put the purchase in a paid invoice.
    #[error("the date falls in a paid invoice")]
    DateInPaidInvoice,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl CardPurchaseError {
    fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<super::CreditCardError> for CardPurchaseError {
    fn from(error: super::CreditCardError) -> Self {
        Self::internal(error)
    }
}

impl From<TransactionError> for CardPurchaseError {
    fn from(error: TransactionError) -> Self {
        match error {
            TransactionError::InvalidCategory(problem) => Self::InvalidCategory(problem),
            TransactionError::NotFound => Self::NotFound,
            TransactionError::Internal(cause) => Self::Internal(cause),
        }
    }
}

/// A card purchase with the invoice it enters (worked out by the ledger from
/// its date and the card's days; never stored).
#[derive(Clone, Debug)]
pub struct CardPurchaseView {
    pub purchase: CardPurchase,
    pub invoice: InvoiceCycle,
}

/// UC15: records a purchase with a credit card. The card must be the user's
/// and active; the Category an active expense one. The purchase enters the
/// invoice by the closing rule and does not change the Current Balance.
pub struct CreateCardPurchaseUseCase {
    purchases: CardPurchaseRepository,
    cards: CreditCardRepository,
    categories: CategoryRepository,
}

impl CreateCardPurchaseUseCase {
    pub fn new(
        purchases: CardPurchaseRepository,
        cards: CreditCardRepository,
        categories: CategoryRepository,
    ) -> Self {
        Self {
            purchases,
            cards,
            categories,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        new: NewCardPurchase,
        today: NaiveDate,
    ) -> Result<CardPurchaseView, CardPurchaseError> {
        ensure_usable_category(
            &self.categories,
            user_id,
            new.category_id,
            TransactionType::Expense,
        )
        .await?;
        let card = self
            .cards
            .find(user_id, new.credit_card_id)
            .await
            .map_err(CardPurchaseError::internal)?
            .ok_or(CardPurchaseError::InvalidCard(CardProblem::NotFound))?;
        if !card.active {
            return Err(CardPurchaseError::InvalidCard(CardProblem::Inactive));
        }
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let cycle = ledger
            .invoice_for_purchase(card.id, new.date, today)
            .ok_or(CardPurchaseError::InvalidCard(CardProblem::NotFound))?;
        if cycle.state == InvoiceState::Paid {
            return Err(CardPurchaseError::DateInPaidInvoice);
        }
        let purchase = self
            .purchases
            .insert(user_id, new)
            .await
            .map_err(CardPurchaseError::internal)?;
        Ok(CardPurchaseView {
            purchase,
            invoice: cycle,
        })
    }
}

/// UC06 for card purchases: edits amount, date, description and Category.
/// Changing the date reallocates the purchase to the invoice the new date
/// belongs to. Nothing in a paid invoice can be edited, and nothing can be
/// moved into one.
pub struct UpdateCardPurchaseUseCase {
    purchases: CardPurchaseRepository,
    cards: CreditCardRepository,
    categories: CategoryRepository,
}

impl UpdateCardPurchaseUseCase {
    pub fn new(
        purchases: CardPurchaseRepository,
        cards: CreditCardRepository,
        categories: CategoryRepository,
    ) -> Self {
        Self {
            purchases,
            cards,
            categories,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: CardPurchaseId,
        changes: CardPurchaseChanges,
        today: NaiveDate,
    ) -> Result<CardPurchaseView, CardPurchaseError> {
        let current = self
            .purchases
            .find(user_id, id)
            .await
            .map_err(CardPurchaseError::internal)?
            .ok_or(CardPurchaseError::NotFound)?;
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let from = view_of(&ledger, current.clone(), today).ok_or(CardPurchaseError::NotFound)?;
        if from.invoice.state == InvoiceState::Paid {
            return Err(CardPurchaseError::InvoicePaid);
        }
        if let Some(category_id) = changes.category_id
            && category_id != current.category_id
        {
            ensure_usable_category(
                &self.categories,
                user_id,
                category_id,
                TransactionType::Expense,
            )
            .await?;
        }
        if let Some(date) = changes.date
            && ledger
                .invoice_for_purchase(current.credit_card_id, date, today)
                .is_some_and(|cycle| cycle.state == InvoiceState::Paid)
        {
            return Err(CardPurchaseError::DateInPaidInvoice);
        }
        let updated = self
            .purchases
            .update(user_id, id, changes)
            .await
            .map_err(CardPurchaseError::internal)?
            .ok_or(CardPurchaseError::NotFound)?;
        // The invoice may have changed with the date, and the amounts of the
        // ledger are stale: only the cycle (dates and state) is needed.
        let invoice = ledger
            .invoice_for_purchase(updated.credit_card_id, updated.date, today)
            .ok_or(CardPurchaseError::NotFound)?;
        Ok(CardPurchaseView {
            purchase: updated,
            invoice,
        })
    }
}

/// UC07 for card purchases: removes a purchase, unless its invoice is paid.
pub struct DeleteCardPurchaseUseCase {
    purchases: CardPurchaseRepository,
    cards: CreditCardRepository,
}

impl DeleteCardPurchaseUseCase {
    pub fn new(purchases: CardPurchaseRepository, cards: CreditCardRepository) -> Self {
        Self { purchases, cards }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: CardPurchaseId,
        today: NaiveDate,
    ) -> Result<(), CardPurchaseError> {
        let current = self
            .purchases
            .find(user_id, id)
            .await
            .map_err(CardPurchaseError::internal)?
            .ok_or(CardPurchaseError::NotFound)?;
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let view = view_of(&ledger, current, today).ok_or(CardPurchaseError::NotFound)?;
        if view.invoice.state == InvoiceState::Paid {
            return Err(CardPurchaseError::InvoicePaid);
        }
        match self.purchases.delete(user_id, id).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(CardPurchaseError::NotFound),
            Err(error) => Err(CardPurchaseError::internal(error)),
        }
    }
}
