use crate::{
    ids::UserId,
    ledger::LedgerFacts,
    repositories::{
        card_purchases::CardPurchaseRepository, credit_cards::CreditCardRepository,
        opening_balance::OpeningBalanceRepository, recurring::RecurringRepository,
        transactions::TransactionRepository,
    },
};

use super::credit_card::card_facts;

/// Reads from the repositories the facts the ledger needs about one user, so
/// that every use case that asks the ledger a question gets them the same way.
/// The reads are independent and run together: a request waits for one round
/// trip to the database, not one per kind of fact.
pub(super) struct LedgerLoader {
    transactions: TransactionRepository,
    opening_balance: OpeningBalanceRepository,
    recurring: RecurringRepository,
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
}

impl LedgerLoader {
    pub(super) fn new(
        transactions: TransactionRepository,
        opening_balance: OpeningBalanceRepository,
        recurring: RecurringRepository,
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
    ) -> Self {
        Self {
            transactions,
            opening_balance,
            recurring,
            cards,
            purchases,
        }
    }

    /// Everything: the Opening Balance, the cash flows, the recurring
    /// Transactions and the cards. Card purchases reach the balance only
    /// through Invoice Payments, which the ledger derives from the cards'
    /// facts.
    pub(super) async fn facts(&self, user_id: UserId) -> Result<LedgerFacts, sqlx::Error> {
        let (opening_balance_cents, cash_flows, schedule) = tokio::try_join!(
            self.opening_balance.find(user_id),
            self.transactions.cash_flows(user_id),
            recurring_and_cards(&self.recurring, &self.cards, &self.purchases, user_id),
        )?;
        Ok(LedgerFacts {
            opening_balance_cents,
            cash_flows,
            ..schedule
        })
    }
}

/// Only the recurring Transactions and the cards, for questions that do not
/// depend on the cash flows or the Opening Balance (the Budgets).
pub(super) async fn recurring_and_cards(
    recurring: &RecurringRepository,
    cards: &CreditCardRepository,
    purchases: &CardPurchaseRepository,
    user_id: UserId,
) -> Result<LedgerFacts, sqlx::Error> {
    let (recurring, (_, cards)) = tokio::try_join!(
        recurring.list(user_id),
        card_facts(cards, purchases, user_id),
    )?;
    Ok(LedgerFacts {
        cards,
        recurring: recurring
            .into_iter()
            .map(|recurring| recurring.definition)
            .collect(),
        ..LedgerFacts::default()
    })
}
