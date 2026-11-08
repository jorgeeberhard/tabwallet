use chrono::NaiveDate;

use crate::{
    ids::{ChargeId, CreditCardId, UserId},
    ledger::{
        Ledger,
        cards::{CardChange, ChargeFact, Invoice, InvoiceCycle, InvoiceState, Month},
    },
    repositories::{
        card_purchases::CardPurchaseRepository,
        categories::CategoryRepository,
        credit_cards::CreditCardRepository,
        invoice_charges::{ChargeChanges, InvoiceCharge, InvoiceChargeRepository, NewCharge},
    },
};

use super::{CreditCardError, credit_card::card_ledger};

/// The description a Charge gets when the user gives none.
pub const DEFAULT_CHARGE_DESCRIPTION: &str = "Juros e encargos";

/// Why a Charge use case failed. Unaware of HTTP.
#[derive(Debug, thiserror::Error)]
pub enum ChargeError {
    /// There is no such credit card, or it belongs to another user.
    #[error("credit card not found")]
    CardNotFound,
    /// The card has no such invoice.
    #[error("invoice not found")]
    InvoiceNotFound,
    /// There is no such Charge, or it belongs to another user.
    #[error("charge not found")]
    NotFound,
    /// The invoice is Paid, so its Charges can no longer be recorded, edited or
    /// deleted.
    #[error("the invoice is paid")]
    InvoicePaid,
    /// The change would leave the invoice with a total below what was already
    /// paid on it early.
    #[error("the change would leave an invoice overpaid")]
    WouldBeOverpaid,
    /// "Juros e encargos" is deactivated, so it cannot take new Charges.
    #[error("the charges category is inactive")]
    CategoryInactive,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl ChargeError {
    fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<CreditCardError> for ChargeError {
    fn from(error: CreditCardError) -> Self {
        match error {
            CreditCardError::NotFound => Self::CardNotFound,
            CreditCardError::InvoiceNotFound => Self::InvoiceNotFound,
            other => Self::internal(other),
        }
    }
}

/// What the user informs of a new Charge; the Category is always the user's
/// "Juros e encargos".
pub struct NewChargeInput {
    /// Cents, greater than zero.
    pub amount_cents: i64,
    pub date: NaiveDate,
    /// [`DEFAULT_CHARGE_DESCRIPTION`] when `None`.
    pub description: Option<String>,
}

/// A Charge with the invoice it is in (dates and state worked out by the
/// ledger; never stored).
#[derive(Clone, Debug)]
pub struct ChargeView {
    pub charge: InvoiceCharge,
    pub invoice: InvoiceCycle,
}

/// A Charge and the invoice, with its amounts, after the Charge was recorded
/// or changed.
pub struct ChargeReceipt {
    pub charge: ChargeView,
    pub invoice: Invoice,
}

/// A Charge with the invoice it belongs to, as the ledger sees it. `None` if
/// the card is unknown to the ledger.
pub(super) fn view_of(
    ledger: &Ledger,
    charge: InvoiceCharge,
    today: NaiveDate,
) -> Option<ChargeView> {
    let invoice = ledger.invoice_cycle(
        charge.credit_card_id,
        Month::of(charge.closing_month),
        today,
    )?;
    Some(ChargeView { charge, invoice })
}

/// Records the interest, penalty and IOF the bank charged on an invoice.
/// It is an expense of the user's "Juros e encargos" Category, linked to the
/// invoice, with a positive amount and a date. It joins the invoice total (so
/// it is paid through the invoice payment) and counts in the month of the
/// invoice's due date. The system never calculates interest. The invoice must
/// exist and not be Paid.
pub struct CreateChargeUseCase {
    charges: InvoiceChargeRepository,
    categories: CategoryRepository,
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
}

impl CreateChargeUseCase {
    pub fn new(
        charges: InvoiceChargeRepository,
        categories: CategoryRepository,
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
    ) -> Self {
        Self {
            charges,
            categories,
            cards,
            purchases,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        card_id: CreditCardId,
        closing_month: Month,
        new: NewChargeInput,
        today: NaiveDate,
    ) -> Result<ChargeReceipt, ChargeError> {
        let (cards, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        if !cards.iter().any(|card| card.id == card_id) {
            return Err(ChargeError::CardNotFound);
        }
        let invoice = ledger
            .invoice(card_id, closing_month, today)
            .ok_or(ChargeError::InvoiceNotFound)?;
        if invoice.cycle.state == InvoiceState::Paid {
            return Err(ChargeError::InvoicePaid);
        }
        let category = self
            .categories
            .find_charges_category(user_id)
            .await
            .map_err(ChargeError::internal)?
            .ok_or_else(|| ChargeError::internal("the user has no charges category"))?;
        if !category.active {
            return Err(ChargeError::CategoryInactive);
        }
        let charge = self
            .charges
            .insert(
                user_id,
                NewCharge {
                    credit_card_id: card_id,
                    closing_month,
                    amount_cents: new.amount_cents,
                    date: new.date,
                    description: new
                        .description
                        .unwrap_or_else(|| DEFAULT_CHARGE_DESCRIPTION.to_string()),
                    category_id: category.id,
                },
            )
            .await
            .map_err(ChargeError::internal)?;
        receipt(&self.cards, &self.purchases, user_id, charge, today).await
    }
}

/// The Charge with the invoice as it stands now.
async fn receipt(
    cards: &CreditCardRepository,
    purchases: &CardPurchaseRepository,
    user_id: UserId,
    charge: InvoiceCharge,
    today: NaiveDate,
) -> Result<ChargeReceipt, ChargeError> {
    let (_, ledger) = card_ledger(cards, purchases, user_id).await?;
    let invoice = ledger
        .invoice(
            charge.credit_card_id,
            Month::of(charge.closing_month),
            today,
        )
        .ok_or(ChargeError::InvoiceNotFound)?;
    Ok(ChargeReceipt {
        charge: ChargeView {
            charge,
            invoice: invoice.cycle,
        },
        invoice,
    })
}

/// The user's Charge and its invoice, or the reason it cannot be changed.
async fn changeable(
    charges: &InvoiceChargeRepository,
    cards: &CreditCardRepository,
    purchases: &CardPurchaseRepository,
    user_id: UserId,
    id: ChargeId,
    today: NaiveDate,
) -> Result<(InvoiceCharge, Ledger), ChargeError> {
    let charge = charges
        .find(user_id, id)
        .await
        .map_err(ChargeError::internal)?
        .ok_or(ChargeError::NotFound)?;
    let (_, ledger) = card_ledger(cards, purchases, user_id).await?;
    let view = view_of(&ledger, charge, today).ok_or(ChargeError::NotFound)?;
    if view.invoice.state == InvoiceState::Paid {
        return Err(ChargeError::InvoicePaid);
    }
    Ok((view.charge, ledger))
}

/// Refuses the change to a Charge (`removed` replaced by `added`, if any)
/// when the invoice would come to less than was paid on it early.
fn ensure_not_overpaid(
    ledger: &Ledger,
    charge: &InvoiceCharge,
    added_cents: Option<i64>,
) -> Result<(), ChargeError> {
    let change = CardChange {
        charges_removed: vec![fact_of(charge, charge.amount_cents)],
        charges_added: added_cents
            .map(|cents| fact_of(charge, cents))
            .into_iter()
            .collect(),
        ..CardChange::default()
    };
    match ledger.overpaid_by(charge.credit_card_id, &change) {
        Some(_) => Err(ChargeError::WouldBeOverpaid),
        None => Ok(()),
    }
}

fn fact_of(charge: &InvoiceCharge, amount_cents: i64) -> ChargeFact {
    ChargeFact {
        closing_month: Month::of(charge.closing_month),
        amount_cents,
    }
}

/// Edits the amount, date and description of a Charge, unless its
/// invoice is Paid.
pub struct UpdateChargeUseCase {
    charges: InvoiceChargeRepository,
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
}

impl UpdateChargeUseCase {
    pub fn new(
        charges: InvoiceChargeRepository,
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
    ) -> Self {
        Self {
            charges,
            cards,
            purchases,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: ChargeId,
        changes: ChargeChanges,
        today: NaiveDate,
    ) -> Result<ChargeReceipt, ChargeError> {
        let (charge, ledger) = changeable(
            &self.charges,
            &self.cards,
            &self.purchases,
            user_id,
            id,
            today,
        )
        .await?;
        ensure_not_overpaid(
            &ledger,
            &charge,
            Some(changes.amount_cents.unwrap_or(charge.amount_cents)),
        )?;
        let updated = self
            .charges
            .update(user_id, id, changes)
            .await
            .map_err(ChargeError::internal)?
            .ok_or(ChargeError::NotFound)?;
        receipt(&self.cards, &self.purchases, user_id, updated, today).await
    }
}

/// Removes a Charge, unless its invoice is Paid.
pub struct DeleteChargeUseCase {
    charges: InvoiceChargeRepository,
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
}

impl DeleteChargeUseCase {
    pub fn new(
        charges: InvoiceChargeRepository,
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
    ) -> Self {
        Self {
            charges,
            cards,
            purchases,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: ChargeId,
        today: NaiveDate,
    ) -> Result<(), ChargeError> {
        let (charge, ledger) = changeable(
            &self.charges,
            &self.cards,
            &self.purchases,
            user_id,
            id,
            today,
        )
        .await?;
        ensure_not_overpaid(&ledger, &charge, None)?;
        match self.charges.delete(user_id, id).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(ChargeError::NotFound),
            Err(error) => Err(ChargeError::internal(error)),
        }
    }
}
