use crate::{
    ids::{CategoryId, UserId},
    repositories::categories::{Category, CategoryRepository, WriteCategoryError},
};

use super::{CategoryError, category_error::name_conflict};

/// UC09: renames a category. Transactions follow, since they point to the
/// category and not to its name.
pub struct RenameCategoryUseCase {
    categories: CategoryRepository,
}

impl RenameCategoryUseCase {
    pub fn new(categories: CategoryRepository) -> Self {
        Self { categories }
    }

    /// `name` is already trimmed and validated by the handler.
    pub async fn execute(
        &self,
        user_id: UserId,
        id: CategoryId,
        name: &str,
    ) -> Result<Category, CategoryError> {
        let current = self
            .categories
            .find(user_id, id)
            .await
            .map_err(CategoryError::internal)?
            .ok_or(CategoryError::NotFound)?;
        match self.categories.rename(user_id, id, name).await {
            Ok(Some(category)) => Ok(category),
            Ok(None) => Err(CategoryError::NotFound),
            Err(WriteCategoryError::NameTaken) => {
                Err(name_conflict(&self.categories, user_id, current.kind, name).await)
            }
            Err(WriteCategoryError::Database(error)) => Err(CategoryError::internal(error)),
        }
    }
}
