use thiserror::Error;

/// Errors returned by the text messaging domain.
#[derive(Debug, Error)]
pub enum TextError {
    /// Underlying common error.
    #[error(transparent)]
    Common(#[from] rs_common::CommonError),
}
