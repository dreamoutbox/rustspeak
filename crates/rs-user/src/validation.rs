use crate::UserError;

const MIN_USERNAME_LEN: usize = 3;
const MAX_USERNAME_LEN: usize = 32;
const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 128;

/// Validates and trims `username`, returning the normalized slice.
///
/// Usernames must be 3..=32 ASCII characters consisting of letters, digits, `_`, or `-`.
pub fn validate_username(username: &str) -> Result<&str, UserError> {
    let trimmed = username.trim();
    if trimmed.len() < MIN_USERNAME_LEN || trimmed.len() > MAX_USERNAME_LEN {
        return Err(UserError::InvalidInput(format!(
            "username must be {MIN_USERNAME_LEN}..={MAX_USERNAME_LEN} characters"
        )));
    }
    let valid_chars = trimmed
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if !valid_chars {
        return Err(UserError::InvalidInput(
            "username may only contain ASCII letters, digits, '_' and '-'".to_string(),
        ));
    }
    Ok(trimmed)
}

/// Validates `password` length constraints (8..=128 bytes).
pub fn validate_password(password: &str) -> Result<(), UserError> {
    if password.len() < MIN_PASSWORD_LEN || password.len() > MAX_PASSWORD_LEN {
        return Err(UserError::InvalidInput(format!(
            "password must be {MIN_PASSWORD_LEN}..={MAX_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

