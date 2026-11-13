use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::RngCore;
use rand::rngs::OsRng;
use rs_common::Claims;
use sha2::{Digest, Sha256};

use crate::{User, UserError};

const DEFAULT_ACCESS_TTL_SECS: u64 = 900;
const DEFAULT_REFRESH_TTL_DAYS: i64 = 30;
const REFRESH_TOKEN_BYTES: usize = 32;

/// Issues and validates JWT access tokens and opaque refresh tokens.
#[derive(Clone)]
pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_ttl_secs: u64,
    refresh_ttl_days: i64,
}

impl TokenService {
    /// Creates a new `TokenService` using the HMAC `jwt_secret`.
    ///
    /// # Example
    /// ```
    /// let tokens = rs_user::TokenService::new("0123456789abcdef0123456789abcdef");
    /// ```
    pub fn new(jwt_secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(jwt_secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(jwt_secret.as_bytes()),
            access_ttl_secs: DEFAULT_ACCESS_TTL_SECS,
            refresh_ttl_days: DEFAULT_REFRESH_TTL_DAYS,
        }
    }

    /// Returns the access token lifetime in seconds.
    pub fn access_ttl_secs(&self) -> u64 {
        self.access_ttl_secs
    }

    /// Computes the expiration timestamp for a newly issued refresh token.
    pub fn refresh_expires_at(&self) -> DateTime<Utc> {
        Utc::now() + Duration::days(self.refresh_ttl_days)
    }

    /// Encodes a signed JWT access token for `user`.
    pub fn issue_access_token(&self, user: &User) -> Result<String, UserError> {
        let now = Utc::now().timestamp().max(0) as usize;
        let exp = now.saturating_add(self.access_ttl_secs as usize);
        let claims = Claims {
            sub: user.id,
            username: user.username.clone(),
            iat: now,
            exp,
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)?;
        Ok(token)
    }

    /// Validates a JWT access `token` and returns its decoded `Claims`.
    pub fn verify_access_token(&self, token: &str) -> Result<Claims, UserError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|_| UserError::InvalidToken)?;
        Ok(data.claims)
    }

    /// Generates a new random refresh token and its SHA-256 hash.
    pub fn generate_refresh_token() -> (String, Vec<u8>) {
        let mut bytes = [0u8; REFRESH_TOKEN_BYTES];
        OsRng.fill_bytes(&mut bytes);
        let raw = URL_SAFE_NO_PAD.encode(bytes);
        let hash = Self::hash_refresh_token(&raw);
        (raw, hash)
    }

    /// Hashes a raw refresh token string with SHA-256 for database lookup/storage.
    pub fn hash_refresh_token(raw: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        hasher.finalize().to_vec()
    }
}

