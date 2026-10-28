use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::extract::{ValidatedJson, trimmed};
use crate::{
    app::AppState,
    auth::AuthUser,
    error::ApiError,
    ids::CategoryId,
    repositories::categories::{Category, CategoryRepository, CategoryType},
    use_cases::{
        CreateCategoryUseCase, DeactivateCategoryUseCase, ListCategoriesUseCase,
        ReactivateCategoryUseCase, RenameCategoryUseCase,
    },
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

pub const MAX_NAME_LEN: u64 = 60;

#[derive(Serialize)]
pub struct CategoryEnvelope {
    category: CategoryResponse,
}

#[derive(Deserialize, Validate)]
pub struct CreateRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, max = MAX_NAME_LEN, code = "name_length"))]
    name: String,
    #[serde(rename = "type")]
    kind: CategoryType,
}

/// `POST /categories` (protected): creates an active category.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateRequest>,
) -> Result<(StatusCode, Json<CategoryEnvelope>), ApiError> {
    let category = CreateCategoryUseCase::new(CategoryRepository::new(state.pool.clone()))
        .execute(auth_user.id, request.kind, &request.name)
        .await?;
    Ok((StatusCode::CREATED, envelope(category)))
}

#[derive(Deserialize, Validate)]
pub struct RenameRequest {
    #[serde(deserialize_with = "trimmed")]
    #[validate(length(min = 1, max = MAX_NAME_LEN, code = "name_length"))]
    name: String,
}

/// A malformed id is a category that does not exist.
fn parse_id(raw: &str) -> Result<CategoryId, ApiError> {
    Uuid::parse_str(raw)
        .map(CategoryId)
        .map_err(|_| ApiError::CategoryNotFound)
}

fn envelope(category: Category) -> Json<CategoryEnvelope> {
    Json(CategoryEnvelope {
        category: category.into(),
    })
}

/// `PATCH /categories/{id}` (protected): renames a category.
pub async fn rename(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<RenameRequest>,
) -> Result<Json<CategoryEnvelope>, ApiError> {
    let category = RenameCategoryUseCase::new(CategoryRepository::new(state.pool.clone()))
        .execute(auth_user.id, parse_id(&id)?, &request.name)
        .await?;
    Ok(envelope(category))
}

/// `POST /categories/{id}/deactivate` (protected): hides the category from
/// new transactions, keeping it on the existing ones.
pub async fn deactivate(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<CategoryEnvelope>, ApiError> {
    let category = DeactivateCategoryUseCase::new(CategoryRepository::new(state.pool.clone()))
        .execute(auth_user.id, parse_id(&id)?)
        .await?;
    Ok(envelope(category))
}

/// `POST /categories/{id}/reactivate` (protected).
pub async fn reactivate(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<CategoryEnvelope>, ApiError> {
    let category = ReactivateCategoryUseCase::new(CategoryRepository::new(state.pool.clone()))
        .execute(auth_user.id, parse_id(&id)?)
        .await?;
    Ok(envelope(category))
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
