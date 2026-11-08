use crate::{
    ids::UserId,
    repositories::categories::{Category, CategoryRepository, CategoryType},
};

use super::CategoryError;

/// UC09: lists the user's categories, active and inactive, optionally only
/// one type and/or only active ones (what a new-transaction form offers).
pub struct ListCategoriesUseCase {
    categories: CategoryRepository,
}

impl ListCategoriesUseCase {
    pub fn new(categories: CategoryRepository) -> Self {
        Self { categories }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        kind: Option<CategoryType>,
        active: Option<bool>,
    ) -> Result<Vec<Category>, CategoryError> {
        self.categories
            .list(user_id, kind, active)
            .await
            .map_err(CategoryError::internal)
    }
}
