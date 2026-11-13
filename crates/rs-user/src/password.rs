use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

use crate::UserError;

/// Hashes a plaintext password using Argon2id with a random salt.
///
/// # Example
/// ```
/// let hash = rs_user::hash_password("correct-horse-battery-staple").unwrap();
/// assert!(rs_user::verify_password("correct-horse-battery-staple", &hash).unwrap());
/// ```
pub fn hash_password(password: &str) -> Result<String, UserError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| UserError::PasswordHash(err.to_string()))?;
    Ok(hash.to_string())
}

/// Verifies a plaintext `password` against a stored Argon2 PHC `hash`.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, UserError> {
    let parsed = PasswordHash::new(hash).map_err(|err| UserError::PasswordHash(err.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
