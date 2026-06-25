use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub totp_secret: Option<String>,
    pub role: String,
    pub created_at: String,
}

/// Safe, public projection of `User` — never exposes password_hash or totp_secret.
#[derive(Debug, Serialize)]
pub struct UserView {
    pub id: String,
    pub username: String,
    pub role: String,
    pub mfa_enabled: bool,
    pub created_at: String,
}

impl From<User> for UserView {
    fn from(user: User) -> Self {
        UserView {
            id: user.id,
            username: user.username,
            role: user.role,
            mfa_enabled: user.totp_secret.is_some(),
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}
