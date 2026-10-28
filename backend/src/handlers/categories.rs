use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::CategoryId,
    repositories::categories::{Category, CategoryRepository, CategoryType},
    use_cases::ListCategoriesUseCase,
};

/// The public view of a category.
#[derive(Serialize)]
pub struct CategoryResponse {
    id: CategoryId,
    name: String,
    #[serde(rename = "type")]
    kind: &'static str,
    active: bool,
    /// The default "Juros e encargos" category.
    is_charges: bool,
    created_at: DateTime<Utc>,
}

impl From<Category> for CategoryResponse {
    fn from(category: Category) -> Self {
        Self {
            id: category.id,
            name: category.name,
            kind: category.kind.as_str(),
            active: category.active,
            is_charges: category.is_charges,
            created_at: category.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct CategoriesEnvelope {
    categories: Vec<CategoryResponse>,
}

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(rename = "type")]
    kind: Option<CategoryType>,
    active: Option<bool>,
}

/// `GET /categories` (protected): the user's categories. Optional query
/// `type=income|expense` and `active=true|false`.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
    params: Result<Query<ListParams>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<CategoriesEnvelope>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::InvalidRequest)?;
    let categories = ListCategoriesUseCase::new(CategoryRepository::new(state.pool.clone()))
        .execute(auth_user.id, params.kind, params.active)
        .await?;
    Ok(Json(CategoriesEnvelope {
        categories: categories.into_iter().map(Into::into).collect(),
    }))
}
