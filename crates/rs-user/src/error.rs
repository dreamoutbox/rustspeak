use thiserror::Error;

/// Errors returned by the user and authentication domain.
#[derive(Debug, Error)]
pub enum UserError {
    /// Underlying common error.
    #[error(transparent)]
    Common(#[from] rs_common::CommonError),
}
