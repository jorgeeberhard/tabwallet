use crate::{
    ids::{CategoryId, UserId},
    repositories::{categories::CategoryRepository, transactions::TransactionType},
};

/// Why a transaction cannot use the Category it was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryProblem {
    /// There is no such category, or it belongs to another user.
    NotFound,
    /// The Category is an income one and the Transaction an expense, or the
    /// other way round.
    TypeMismatch,
    /// The Category was deactivated, so it cannot take new transactions.
    Inactive,
}

/// Why a transaction use case failed. Unaware of HTTP: the handler layer
/// decides the status code.
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    /// There is no such transaction, or it belongs to another user.
    #[error("transaction not found")]
    NotFound,
    #[error("the category cannot be used: {0:?}")]
    InvalidCategory(CategoryProblem),
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl TransactionError {
    pub fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}

/// Checks that the user's Category can take a Transaction of this type: it
/// exists for this user, has the same type and is active.
pub(super) async fn ensure_usable_category(
    categories: &CategoryRepository,
    user_id: UserId,
    category_id: CategoryId,
    kind: TransactionType,
) -> Result<(), TransactionError> {
    let category = categories
        .find(user_id, category_id)
        .await
        .map_err(TransactionError::internal)?
        .ok_or(TransactionError::InvalidCategory(CategoryProblem::NotFound))?;
    if category.kind != kind {
        return Err(TransactionError::InvalidCategory(
            CategoryProblem::TypeMismatch,
        ));
    }
    if !category.active {
        return Err(TransactionError::InvalidCategory(CategoryProblem::Inactive));
    }
    Ok(())
}
