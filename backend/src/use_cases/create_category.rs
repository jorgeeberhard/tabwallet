use crate::{
    ids::UserId,
    repositories::categories::{Category, CategoryRepository, CategoryType, WriteCategoryError},
};

use super::{CategoryError, category_error::name_conflict};

/// UC09: creates a category. If the name belongs to a deactivated category of
/// the same type it fails with `InactiveNameExists`, so the user can
/// reactivate that one instead.
pub struct CreateCategoryUseCase {
    categories: CategoryRepository,
}

impl CreateCategoryUseCase {
    pub fn new(categories: CategoryRepository) -> Self {
        Self { categories }
    }

    /// `name` is already trimmed and validated by the handler.
    pub async fn execute(
        &self,
        user_id: UserId,
        kind: CategoryType,
        name: &str,
    ) -> Result<Category, CategoryError> {
        match self.categories.insert(user_id, kind, name).await {
            Ok(category) => Ok(category),
            Err(WriteCategoryError::NameTaken) => {
                Err(name_conflict(&self.categories, user_id, kind, name).await)
            }
            Err(WriteCategoryError::Database(error)) => Err(CategoryError::internal(error)),
        }
    }
}
