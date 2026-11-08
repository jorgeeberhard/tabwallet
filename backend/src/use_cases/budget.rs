use chrono::NaiveDate;

use crate::{
    ids::{CategoryId, UserId},
    ledger::{BudgetFacts, BudgetStatus, Ledger, LedgerFacts, cards::Month},
    repositories::{
        budgets::{BudgetRepository, StoredBudget},
        card_purchases::CardPurchaseRepository,
        categories::{CategoryRepository, CategoryType},
        credit_cards::CreditCardRepository,
        recurring::RecurringRepository,
        spending::SpendingRepository,
    },
};

use super::{CategoryProblem, credit_card::card_facts};

/// Why a budget use case failed. Unaware of HTTP: the handler layer decides
/// the status code.
#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    /// The Category cannot take a Budget: unknown, not an expense one, or
    /// deactivated.
    #[error("the category cannot be used: {0:?}")]
    InvalidCategory(CategoryProblem),
    /// The Category has no Budget in force to remove.
    #[error("budget not found")]
    NotFound,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl BudgetError {
    fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

/// The Category a Budget belongs to.
#[derive(Clone, Debug)]
pub struct BudgetCategory {
    pub id: CategoryId,
    pub name: String,
    pub active: bool,
}

/// A Budget as the user sees it in a month: the limit in force and the month
/// it started in.
#[derive(Clone, Debug)]
pub struct BudgetView {
    pub category: BudgetCategory,
    pub limit_cents: i64,
    pub effective_from: Month,
}

fn view_in(stored: &StoredBudget, month: Month) -> Option<BudgetView> {
    let (limit_cents, effective_from) = stored.budget.limit_in(month)?;
    Some(BudgetView {
        category: BudgetCategory {
            id: stored.budget.category_id,
            name: stored.category_name.clone(),
            active: stored.budget.category_active,
        },
        limit_cents,
        effective_from,
    })
}

/// Defines or changes the Budget of an expense Category. The new limit
/// applies from the month of `today` on; earlier months keep the limit they
/// had.
pub struct SetBudgetUseCase {
    budgets: BudgetRepository,
    categories: CategoryRepository,
}

impl SetBudgetUseCase {
    pub fn new(budgets: BudgetRepository, categories: CategoryRepository) -> Self {
        Self {
            budgets,
            categories,
        }
    }

    /// `limit_cents` has already been checked to be greater than zero. The
    /// Category must be the user's, an active expense one.
    pub async fn execute(
        &self,
        user_id: UserId,
        category_id: CategoryId,
        limit_cents: i64,
        today: NaiveDate,
    ) -> Result<BudgetView, BudgetError> {
        let category = self
            .categories
            .find(user_id, category_id)
            .await
            .map_err(BudgetError::internal)?
            .ok_or(BudgetError::InvalidCategory(CategoryProblem::NotFound))?;
        if category.kind != CategoryType::Expense {
            return Err(BudgetError::InvalidCategory(CategoryProblem::TypeMismatch));
        }
        if !category.active {
            return Err(BudgetError::InvalidCategory(CategoryProblem::Inactive));
        }
        let from = Month::of(today);
        self.budgets
            .set_limit(user_id, category_id, from, limit_cents)
            .await
            .map_err(BudgetError::internal)?;
        Ok(BudgetView {
            category: BudgetCategory {
                id: category.id,
                name: category.name,
                active: category.active,
            },
            limit_cents,
            effective_from: from,
        })
    }
}

/// The Budgets in force in the month of `today`, by Category name. Those of
/// deactivated Categories are kept and listed, marked as such.
pub struct ListBudgetsUseCase {
    budgets: BudgetRepository,
}

impl ListBudgetsUseCase {
    pub fn new(budgets: BudgetRepository) -> Self {
        Self { budgets }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        today: NaiveDate,
    ) -> Result<Vec<BudgetView>, BudgetError> {
        let month = Month::of(today);
        Ok(self
            .budgets
            .list(user_id)
            .await
            .map_err(BudgetError::internal)?
            .iter()
            .filter_map(|stored| view_in(stored, month))
            .collect())
    }
}

/// Removes the Budget of a Category from the month of `today` on.
/// Earlier months keep the limit they had.
pub struct RemoveBudgetUseCase {
    budgets: BudgetRepository,
}

impl RemoveBudgetUseCase {
    pub fn new(budgets: BudgetRepository) -> Self {
        Self { budgets }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        category_id: CategoryId,
        today: NaiveDate,
    ) -> Result<(), BudgetError> {
        let month = Month::of(today);
        let in_force = self
            .budgets
            .list(user_id)
            .await
            .map_err(BudgetError::internal)?
            .iter()
            .any(|stored| {
                stored.budget.category_id == category_id && stored.budget.limit_in(month).is_some()
            });
        if !in_force {
            return Err(BudgetError::NotFound);
        }
        self.budgets
            .end_from(user_id, category_id, month)
            .await
            .map_err(BudgetError::internal)
    }
}

/// One Category against its Budget in a Reference Month.
#[derive(Clone, Debug)]
pub struct BudgetStatusLine {
    pub category: BudgetCategory,
    pub status: BudgetStatus,
}

/// Where the Budgets stand in one Reference Month.
#[derive(Clone, Debug)]
pub struct BudgetReport {
    pub month: Month,
    /// By Category name.
    pub lines: Vec<BudgetStatusLine>,
}

impl BudgetReport {
    /// How many Categories have a Budget Alert in this month.
    pub fn exceeded_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| line.status.exceeded)
            .count()
    }
}

/// The spending of each Category with a Budget in a Reference Month
/// against its limit. The Budget Alert is not stored: it is derived here on
/// every read, from the expenses (Projections included) at that moment.
pub struct GetBudgetStatusUseCase {
    budgets: BudgetRepository,
    spending: SpendingRepository,
    recurring: RecurringRepository,
    cards: CreditCardRepository,
    purchases: CardPurchaseRepository,
}

impl GetBudgetStatusUseCase {
    pub fn new(
        budgets: BudgetRepository,
        spending: SpendingRepository,
        recurring: RecurringRepository,
        cards: CreditCardRepository,
        purchases: CardPurchaseRepository,
    ) -> Self {
        Self {
            budgets,
            spending,
            recurring,
            cards,
            purchases,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        month: Month,
    ) -> Result<BudgetReport, BudgetError> {
        let stored = self
            .budgets
            .list(user_id)
            .await
            .map_err(BudgetError::internal)?;
        // Nothing to compare with: skip reading every expense.
        if stored
            .iter()
            .all(|stored| stored.budget.limit_in(month).is_none())
        {
            return Ok(BudgetReport {
                month,
                lines: Vec::new(),
            });
        }
        let spending = self
            .spending
            .facts(user_id)
            .await
            .map_err(BudgetError::internal)?;
        let recurring = self
            .recurring
            .list(user_id)
            .await
            .map_err(BudgetError::internal)?
            .into_iter()
            .map(|recurring| recurring.definition)
            .collect();
        let (_, cards) = card_facts(&self.cards, &self.purchases, user_id)
            .await
            .map_err(BudgetError::internal)?;

        let ledger = Ledger::new(LedgerFacts {
            recurring,
            cards,
            ..LedgerFacts::default()
        })
        .with_budget_facts(BudgetFacts {
            spending,
            budgets: stored.iter().map(|stored| stored.budget.clone()).collect(),
        });

        let lines = ledger
            .budget_status(month)
            .into_iter()
            .filter_map(|status| {
                let name = stored
                    .iter()
                    .find(|stored| stored.budget.category_id == status.category_id)?
                    .category_name
                    .clone();
                Some(BudgetStatusLine {
                    category: BudgetCategory {
                        id: status.category_id,
                        name,
                        active: status.category_active,
                    },
                    status,
                })
            })
            .collect();
        Ok(BudgetReport { month, lines })
    }
}
