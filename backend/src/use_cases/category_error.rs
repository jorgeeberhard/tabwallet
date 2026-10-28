use crate::ids::CategoryId;

/// Why a category use case failed. Unaware of HTTP: the handler layer decides
/// the status code.
#[derive(Debug, thiserror::Error)]
pub enum CategoryError {
    /// There is no such category, or it belongs to another user.
    #[error("category not found")]
    NotFound,
    /// An active category of the same type already has this name.
    #[error("an active category with this name already exists")]
    NameTaken,
    /// A deactivated category of the same type has this name; the user can
    /// reactivate it instead of creating a duplicate.
    #[error("a deactivated category with this name exists")]
    InactiveNameExists(CategoryId),
    /// "Juros e encargos" cannot be deactivated while Charges exist.
    #[error("the category still has charges")]
    HasCharges,
    #[error("unexpected failure: {0}")]
    Internal(String),
}

impl CategoryError {
    pub fn internal(cause: impl std::fmt::Display) -> Self {
        Self::Internal(cause.to_string())
    }
}
