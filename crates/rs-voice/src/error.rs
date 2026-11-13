use thiserror::Error;

/// Errors returned by the voice and SFU domain.
#[derive(Debug, Error)]
pub enum VoiceError {
    /// Underlying common error.
    #[error(transparent)]
    Common(#[from] rs_common::CommonError),
}
