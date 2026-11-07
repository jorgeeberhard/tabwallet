use std::collections::HashMap;

use chrono::NaiveDate;

use crate::{
    ids::{InstallmentPurchaseId, UserId},
    ledger::{
        Ledger,
        cards::{CardFacts, InvoiceCycle, InvoiceState, PlannedInstallment},
    },
    repositories::{
        card_purchases::CardPurchaseRepository,
        categories::CategoryRepository,
        credit_cards::CreditCardRepository,
        installment_purchases::{
            Allocation, Installment, InstallmentPurchase, InstallmentPurchaseChanges,
            InstallmentPurchaseRepository, NewInstallmentPurchase,
        },
        transactions::TransactionType,
    },
};

use super::{
    CardProblem, CategoryProblem, TransactionError, credit_card::card_ledger,
    transaction_error::ensure_usable_category,
};

/// Why an installment-purchase use case failed. Unaware of HTTP.
#[derive(Debug, thiserror::Error)]
pub enum InstallmentPurchaseError {
    /// There is no such purchase, or it belongs to another user.
    #[error("installment purchase not found")]
    NotFound,
    #[error("the category cannot be used: {0:?}")]
    InvalidCategory(CategoryProblem),
    #[error("the card cannot be used: {0:?}")]
    InvalidCard(CardProblem),
    /// A field breaks a rule the input validation could not see.
    #[error("{field} is invalid: {rule}")]
    Invalid {
        field: &'static str,
        rule: &'static str,
    },
    /// The date would put the first Installment in a paid invoice.
    #[error("the date falls in a paid invoice")]
    DateInPaidInvoice,
    /// Some Installment is already in a paid invoice, so only the Category
    /// can change.
    #[error("some installments are already in a paid invoice")]
    InstallmentsPaid,
    /// Every Installment is in a paid invoice: nothing is left to cancel.
    #[error("every installment is in a paid invoice")]
    AllPaid,
    /// Some Installment was anticipated, which cannot be undone, so only the
    /// Category can change; or nothing is left to cancel because what remains
    /// was anticipated.
    #[error("some installments were anticipated")]
    InstallmentsAnticipated,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl InstallmentPurchaseError {
    fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

impl From<super::CreditCardError> for InstallmentPurchaseError {
    fn from(error: super::CreditCardError) -> Self {
        Self::internal(error)
    }
}

impl From<TransactionError> for InstallmentPurchaseError {
    fn from(error: TransactionError) -> Self {
        match error {
            TransactionError::InvalidCategory(problem) => Self::InvalidCategory(problem),
            TransactionError::NotFound => Self::NotFound,
            TransactionError::Internal(cause) => Self::Internal(cause),
        }
    }
}

/// An Installment with the invoice it is in (its dates and state, worked out
/// by the ledger on every read).
#[derive(Clone, Debug)]
pub struct InstallmentView {
    pub installment: Installment,
    pub invoice: InvoiceCycle,
}

/// An installment purchase with its Installments, oldest number first.
#[derive(Clone, Debug)]
pub struct InstallmentPurchaseView {
    pub purchase: InstallmentPurchase,
    pub installments: Vec<InstallmentView>,
}

impl InstallmentPurchaseView {
    /// Some Installment is in a paid invoice: the purchase can no longer be
    /// recalculated, only have its Category changed and its remaining
    /// Installments cancelled.
    pub fn has_paid_installments(&self) -> bool {
        self.installments
            .iter()
            .any(|view| view.invoice.state == InvoiceState::Paid)
    }

    /// Some Installment was brought to another invoice by an Installment
    /// Anticipation. Irreversible, so the purchase is treated like one with
    /// paid Installments: only the Category can change and the remaining
    /// Installments can be cancelled.
    pub fn has_anticipated_installments(&self) -> bool {
        self.installments
            .iter()
            .any(|view| view.installment.is_anticipated())
    }

    /// Cents of Discount the purchase received in Installment Anticipations.
    pub fn discount_cents(&self) -> i64 {
        self.installments
            .iter()
            .map(|view| view.installment.discount_cents)
            .sum()
    }

    /// The Installments a cancellation takes away: those neither paid nor
    /// anticipated.
    fn cancellable_ids(&self) -> Vec<crate::ids::InstallmentId> {
        self.installments
            .iter()
            .filter(|view| {
                view.invoice.state != InvoiceState::Paid && !view.installment.is_anticipated()
            })
            .map(|view| view.installment.id)
            .collect()
    }
}

/// One Installment as an item of the invoice it is in, with its purchase.
#[derive(Clone, Debug)]
pub struct InvoiceInstallment {
    pub purchase: InstallmentPurchase,
    pub installment: InstallmentView,
    /// Some Installment of the purchase (not only this one) is in a paid
    /// invoice.
    pub purchase_has_paid_installments: bool,
    /// Some Installment of the purchase was anticipated.
    pub purchase_has_anticipated_installments: bool,
}

/// The Installments in the invoice of `card_id` closing in `closing_month`,
/// by purchase date, newest first, then by number.
pub(super) async fn invoice_installments(
    repository: &InstallmentPurchaseRepository,
    ledger: &Ledger,
    user_id: UserId,
    card_id: crate::ids::CreditCardId,
    closing_month: crate::ledger::cards::Month,
    today: NaiveDate,
) -> Result<Vec<InvoiceInstallment>, InstallmentPurchaseError> {
    let purchases = repository
        .list_for_card(user_id, card_id)
        .await
        .map_err(InstallmentPurchaseError::internal)?;
    let views = views_of(repository, ledger, user_id, purchases, today).await?;
    Ok(views
        .into_iter()
        .flat_map(|view| {
            let has_paid = view.has_paid_installments();
            let has_anticipated = view.has_anticipated_installments();
            let purchase = view.purchase;
            view.installments
                .into_iter()
                .filter(move |installment| installment.invoice.closing_month == closing_month)
                .map(move |installment| InvoiceInstallment {
                    purchase: purchase.clone(),
                    installment,
                    purchase_has_paid_installments: has_paid,
                    purchase_has_anticipated_installments: has_anticipated,
                })
        })
        .collect())
}

/// Gives the card's Installments the invoices its new days put them in.
///
/// A single purchase follows its date, so changing the closing day moves it
/// by itself. An Installment is stored with its invoice, so the purchases
/// whose Installments are all still unpaid are planned again with the card's
/// days as they are now (the same rule as when they were launched); one with
/// a paid Installment is history and stays as it is.
pub(super) async fn reallocate_for_card(
    repository: &InstallmentPurchaseRepository,
    user_id: UserId,
    card: &CardFacts,
    today: NaiveDate,
) -> Result<(), InstallmentPurchaseError> {
    let purchases = repository
        .list_for_card(user_id, card.id)
        .await
        .map_err(InstallmentPurchaseError::internal)?;
    let ids: Vec<InstallmentPurchaseId> = purchases.iter().map(|purchase| purchase.id).collect();
    let installments = repository
        .installments(user_id, &ids)
        .await
        .map_err(InstallmentPurchaseError::internal)?;
    for purchase in purchases {
        let any_paid = installments
            .iter()
            .filter(|installment| installment.installment_purchase_id == purchase.id)
            .any(|installment| {
                card.cycle_of(installment.invoice_month(), today).state == InvoiceState::Paid
            });
        if any_paid || purchase.cancelled {
            continue;
        }
        let Some(plan) = card.plan_installments(
            purchase.date,
            purchase.total_cents,
            purchase.installment_count as u32,
            today,
        ) else {
            continue;
        };
        if plan
            .first()
            .is_some_and(|first| first.cycle.state == InvoiceState::Paid)
        {
            continue;
        }
        let own: Vec<&Installment> = installments
            .iter()
            .filter(|installment| installment.installment_purchase_id == purchase.id)
            .collect();
        if own.iter().any(|installment| installment.is_anticipated()) {
            // An anticipated Installment is history and stays where it is
            // (with its reduced amount). The others take the invoices the
            // new days give them; their amounts do not change.
            let moves: Vec<_> = own
                .iter()
                .filter(|installment| !installment.is_anticipated())
                .filter_map(|installment| {
                    let planned = plan
                        .iter()
                        .find(|planned| planned.number as i32 == installment.number)?;
                    Some((installment.id, planned.cycle))
                })
                .collect();
            if moves
                .iter()
                .any(|(_, cycle)| cycle.state == InvoiceState::Paid)
            {
                continue;
            }
            let moves: Vec<_> = moves
                .into_iter()
                .map(|(id, cycle)| (id, cycle.closing_month))
                .collect();
            repository
                .move_installments(user_id, &moves)
                .await
                .map_err(InstallmentPurchaseError::internal)?;
            continue;
        }
        repository
            .update(
                user_id,
                purchase.id,
                InstallmentPurchaseChanges::default(),
                Some(&allocations_of(&plan)),
            )
            .await
            .map_err(InstallmentPurchaseError::internal)?;
    }
    Ok(())
}

/// The views of these purchases (same order); one whose card is unknown to
/// the ledger is left out.
pub(super) async fn views_of(
    repository: &InstallmentPurchaseRepository,
    ledger: &Ledger,
    user_id: UserId,
    purchases: Vec<InstallmentPurchase>,
    today: NaiveDate,
) -> Result<Vec<InstallmentPurchaseView>, InstallmentPurchaseError> {
    let ids: Vec<InstallmentPurchaseId> = purchases.iter().map(|purchase| purchase.id).collect();
    let mut grouped: HashMap<InstallmentPurchaseId, Vec<Installment>> = HashMap::new();
    for installment in repository
        .installments(user_id, &ids)
        .await
        .map_err(InstallmentPurchaseError::internal)?
    {
        grouped
            .entry(installment.installment_purchase_id)
            .or_default()
            .push(installment);
    }
    Ok(purchases
        .into_iter()
        .filter_map(|purchase| {
            let installments = grouped
                .remove(&purchase.id)
                .unwrap_or_default()
                .into_iter()
                .map(|installment| {
                    let invoice = ledger.invoice_cycle(
                        purchase.credit_card_id,
                        installment.invoice_month(),
                        today,
                    )?;
                    Some(InstallmentView {
                        installment,
                        invoice,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(InstallmentPurchaseView {
                purchase,
                installments,
            })
        })
        .collect())
}

/// The view of one purchase.
async fn view_of(
    repository: &InstallmentPurchaseRepository,
    ledger: &Ledger,
    user_id: UserId,
    purchase: InstallmentPurchase,
    today: NaiveDate,
) -> Result<InstallmentPurchaseView, InstallmentPurchaseError> {
    views_of(repository, ledger, user_id, vec![purchase], today)
        .await?
        .pop()
        .ok_or(InstallmentPurchaseError::NotFound)
}

/// The rows to store for a plan.
fn allocations_of(plan: &[PlannedInstallment]) -> Vec<Allocation> {
    plan.iter()
        .map(|planned| Allocation {
            number: planned.number,
            amount_cents: planned.amount_cents,
            invoice_month: planned.cycle.closing_month,
        })
        .collect()
}

/// Splits the purchase and checks that it may start where the closing rule
/// puts it: a purchase cannot enter a paid invoice (the first Installment is
/// in the earliest invoice, so the later ones are not paid either).
fn plan(
    ledger: &Ledger,
    card_id: crate::ids::CreditCardId,
    date: NaiveDate,
    total_cents: i64,
    count: u32,
    today: NaiveDate,
) -> Result<Vec<PlannedInstallment>, InstallmentPurchaseError> {
    let plan = ledger
        .plan_installments(card_id, date, total_cents, count, today)
        .ok_or(InstallmentPurchaseError::Invalid {
            field: "total_cents",
            rule: "total_below_installments",
        })?;
    if plan
        .first()
        .is_some_and(|first| first.cycle.state == InvoiceState::Paid)
    {
        return Err(InstallmentPurchaseError::DateInPaidInvoice);
    }
    Ok(plan)
}

/// UC18: records a purchase with a credit card split in Installments. The
/// user informs the total (already with interest, if any) and the number of
/// Installments; the ledger splits it in exact cents and gives each
/// Installment a consecutive invoice. It does not change the Current Balance.
pub struct CreateInstallmentPurchaseUseCase {
    installment_purchases: InstallmentPurchaseRepository,
    purchases: CardPurchaseRepository,
    cards: CreditCardRepository,
    categories: CategoryRepository,
}

impl CreateInstallmentPurchaseUseCase {
    pub fn new(
        installment_purchases: InstallmentPurchaseRepository,
        purchases: CardPurchaseRepository,
        cards: CreditCardRepository,
        categories: CategoryRepository,
    ) -> Self {
        Self {
            installment_purchases,
            purchases,
            cards,
            categories,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        new: NewInstallmentPurchase,
        today: NaiveDate,
    ) -> Result<InstallmentPurchaseView, InstallmentPurchaseError> {
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
            .map_err(InstallmentPurchaseError::internal)?
            .ok_or(InstallmentPurchaseError::InvalidCard(CardProblem::NotFound))?;
        if !card.active {
            return Err(InstallmentPurchaseError::InvalidCard(CardProblem::Inactive));
        }
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let plan = plan(
            &ledger,
            card.id,
            new.date,
            new.total_cents,
            new.installment_count,
            today,
        )?;
        let purchase = self
            .installment_purchases
            .insert(user_id, new, &allocations_of(&plan))
            .await
            .map_err(InstallmentPurchaseError::internal)?;
        view_of(
            &self.installment_purchases,
            &ledger,
            user_id,
            purchase,
            today,
        )
        .await
    }
}

/// UC06 for installment purchases: edits total, number of Installments, date,
/// description and Category. While no Installment is in a paid invoice
/// everything is recalculated and reallocated; otherwise only the Category
/// can change.
pub struct UpdateInstallmentPurchaseUseCase {
    installment_purchases: InstallmentPurchaseRepository,
    purchases: CardPurchaseRepository,
    cards: CreditCardRepository,
    categories: CategoryRepository,
}

impl UpdateInstallmentPurchaseUseCase {
    pub fn new(
        installment_purchases: InstallmentPurchaseRepository,
        purchases: CardPurchaseRepository,
        cards: CreditCardRepository,
        categories: CategoryRepository,
    ) -> Self {
        Self {
            installment_purchases,
            purchases,
            cards,
            categories,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: InstallmentPurchaseId,
        changes: InstallmentPurchaseChanges,
        today: NaiveDate,
    ) -> Result<InstallmentPurchaseView, InstallmentPurchaseError> {
        let current = self
            .installment_purchases
            .find(user_id, id)
            .await
            .map_err(InstallmentPurchaseError::internal)?
            .ok_or(InstallmentPurchaseError::NotFound)?;
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let view = view_of(
            &self.installment_purchases,
            &ledger,
            user_id,
            current.clone(),
            today,
        )
        .await?;
        // Paid and anticipated Installments are history: the purchase cannot
        // be recalculated, only its Category can change.
        let locked = view.has_paid_installments() || view.has_anticipated_installments();
        if locked
            && (changes.total_cents.is_some()
                || changes.installment_count.is_some()
                || changes.date.is_some()
                || changes.description.is_some())
        {
            return Err(if view.has_paid_installments() {
                InstallmentPurchaseError::InstallmentsPaid
            } else {
                InstallmentPurchaseError::InstallmentsAnticipated
            });
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
        let allocations = if locked {
            None
        } else {
            let plan = plan(
                &ledger,
                current.credit_card_id,
                changes.date.unwrap_or(current.date),
                changes.total_cents.unwrap_or(current.total_cents),
                changes
                    .installment_count
                    .unwrap_or(current.installment_count as u32),
                today,
            )?;
            Some(allocations_of(&plan))
        };
        let updated = self
            .installment_purchases
            .update(user_id, id, changes, allocations.as_deref())
            .await
            .map_err(InstallmentPurchaseError::internal)?
            .ok_or(InstallmentPurchaseError::NotFound)?;
        view_of(
            &self.installment_purchases,
            &ledger,
            user_id,
            updated,
            today,
        )
        .await
    }
}

/// What deleting an installment purchase did.
pub enum CancelOutcome {
    /// No Installment was in a paid invoice: the whole purchase is gone.
    Deleted,
    /// Some were paid: those stay, the remaining ones were cancelled.
    RemainingCancelled(InstallmentPurchaseView),
}

/// UC07 for installment purchases: removes the purchase while no Installment
/// is in a paid invoice; with some paid, cancels the remaining ones and keeps
/// the paid ones as they are.
pub struct CancelInstallmentPurchaseUseCase {
    installment_purchases: InstallmentPurchaseRepository,
    purchases: CardPurchaseRepository,
    cards: CreditCardRepository,
}

impl CancelInstallmentPurchaseUseCase {
    pub fn new(
        installment_purchases: InstallmentPurchaseRepository,
        purchases: CardPurchaseRepository,
        cards: CreditCardRepository,
    ) -> Self {
        Self {
            installment_purchases,
            purchases,
            cards,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: InstallmentPurchaseId,
        today: NaiveDate,
    ) -> Result<CancelOutcome, InstallmentPurchaseError> {
        let current = self
            .installment_purchases
            .find(user_id, id)
            .await
            .map_err(InstallmentPurchaseError::internal)?
            .ok_or(InstallmentPurchaseError::NotFound)?;
        let (_, ledger) = card_ledger(&self.cards, &self.purchases, user_id).await?;
        let view = view_of(
            &self.installment_purchases,
            &ledger,
            user_id,
            current,
            today,
        )
        .await?;
        // Paid and anticipated Installments stay; the others are cancelled.
        let remaining = view.cancellable_ids();
        if remaining.is_empty() {
            let anticipated_and_unpaid = view.installments.iter().any(|installment| {
                installment.installment.is_anticipated()
                    && installment.invoice.state != InvoiceState::Paid
            });
            return Err(if anticipated_and_unpaid {
                InstallmentPurchaseError::InstallmentsAnticipated
            } else {
                InstallmentPurchaseError::AllPaid
            });
        }
        if !view.has_paid_installments() && !view.has_anticipated_installments() {
            return match self.installment_purchases.delete(user_id, id).await {
                Ok(true) => Ok(CancelOutcome::Deleted),
                Ok(false) => Err(InstallmentPurchaseError::NotFound),
                Err(error) => Err(InstallmentPurchaseError::internal(error)),
            };
        }
        let cancelled = self
            .installment_purchases
            .cancel_installments(user_id, id, &remaining)
            .await
            .map_err(InstallmentPurchaseError::internal)?
            .ok_or(InstallmentPurchaseError::NotFound)?;
        // The ledger's amounts are stale now, but a view only needs the
        // dates and state of invoices.
        Ok(CancelOutcome::RemainingCancelled(
            view_of(
                &self.installment_purchases,
                &ledger,
                user_id,
                cancelled,
                today,
            )
            .await?,
        ))
    }
}
