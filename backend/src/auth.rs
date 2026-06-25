use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::User;

pub const ACCESS_TOKEN_TTL_SECONDS: i64 = 15 * 60;
pub const REFRESH_TOKEN_TTL_DAYS: i64 = 7;
const TOTP_ISSUER: &str = "CardFlow";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    pub role: String,
}

pub fn jwt_secret() -> Result<Vec<u8>, AppError> {
    std::env::var("JWT_SECRET")
        .map(|s| s.into_bytes())
        .map_err(|_| AppError::Internal("JWT_SECRET is not configured".into()))
}

pub fn cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|v| v != "false")
        .unwrap_or(true)
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::Internal(format!("failed to hash password: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("stored password hash is invalid: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn create_access_token(user: &User, secret: &[u8]) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user.id.clone(),
        role: user.role.clone(),
        iat: now,
        exp: now + ACCESS_TOKEN_TTL_SECONDS,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| AppError::Internal(format!("failed to create access token: {e}")))
}

pub fn verify_access_token(token: &str, secret: &[u8]) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized("invalid or expired access token".into()))
}

pub fn generate_refresh_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn build_totp(secret_bytes: Vec<u8>, username: &str) -> Result<TOTP, AppError> {
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some(TOTP_ISSUER.to_string()),
        username.to_string(),
    )
    .map_err(|e| AppError::Internal(format!("failed to build totp: {e}")))
}

/// Generates a fresh, unpersisted TOTP secret for enrollment. The caller must echo the
/// returned secret back via `check_totp` in `/auth/mfa/verify` since nothing is stored
/// until the code is confirmed.
pub fn generate_totp_secret(username: &str) -> Result<(String, String), AppError> {
    let secret_bytes = Secret::generate_secret()
        .to_bytes()
        .map_err(|e| AppError::Internal(format!("failed to generate totp secret: {e}")))?;
    let totp = build_totp(secret_bytes, username)?;
    Ok((totp.get_secret_base32(), totp.get_url()))
}

pub fn check_totp(secret_base32: &str, username: &str, code: &str) -> Result<bool, AppError> {
    let secret_bytes = Secret::Encoded(secret_base32.to_string())
        .to_bytes()
        .map_err(|e| AppError::BadRequest(format!("invalid totp secret: {e}")))?;
    let totp = build_totp(secret_bytes, username)?;
    totp.check_current(code)
        .map_err(|e| AppError::Internal(format!("failed to check totp code: {e}")))
}
