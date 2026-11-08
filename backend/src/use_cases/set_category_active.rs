use crate::{
    ids::{CategoryId, UserId},
    repositories::categories::{Category, CategoryRepository},
};

use super::CategoryError;

/// UC09: deactivates a category. It stays on the transactions that already
/// use it but is no longer offered for new ones, and its name stays taken.
/// Deactivating an inactive category changes nothing.
pub struct DeactivateCategoryUseCase {
    categories: CategoryRepository,
}

impl DeactivateCategoryUseCase {
    pub fn new(categories: CategoryRepository) -> Self {
        Self { categories }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: CategoryId,
    ) -> Result<Category, CategoryError> {
        let category = self
            .categories
            .find(user_id, id)
            .await
            .map_err(CategoryError::internal)?
            .ok_or(CategoryError::NotFound)?;
        if !category.active {
            return Ok(category);
        }
        // "Juros e encargos" must keep working while Charges point to it.
        if category.is_charges
            && self
                .categories
                .has_charges(user_id, id)
                .await
                .map_err(CategoryError::internal)?
        {
            return Err(CategoryError::HasCharges);
        }
        self.categories
            .set_active(user_id, id, false)
            .await
            .map_err(CategoryError::internal)?
            .ok_or(CategoryError::NotFound)
    }
}

/// UC09: reactivates a category, which comes back with its budget (if any).
/// Reactivating an active category changes nothing.
pub struct ReactivateCategoryUseCase {
    categories: CategoryRepository,
}

impl ReactivateCategoryUseCase {
    pub fn new(categories: CategoryRepository) -> Self {
        Self { categories }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        id: CategoryId,
    ) -> Result<Category, CategoryError> {
        self.categories
            .set_active(user_id, id, true)
            .await
            .map_err(CategoryError::internal)?
            .ok_or(CategoryError::NotFound)
    }
}
