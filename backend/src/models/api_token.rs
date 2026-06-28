use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ApiToken {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub token_hash: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Safe, public projection of `ApiToken` — never exposes token_hash.
#[derive(Debug, Serialize)]
pub struct ApiTokenView {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

impl From<ApiToken> for ApiTokenView {
    fn from(token: ApiToken) -> Self {
        ApiTokenView {
            id: token.id,
            name: token.name,
            created_at: token.created_at,
            last_used_at: token.last_used_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateApiToken {
    pub name: String,
}

/// Returned only once, at creation — the raw token is never retrievable again.
#[derive(Debug, Serialize)]
pub struct CreateApiTokenResponse {
    pub id: String,
    pub name: String,
    pub token: String,
    pub created_at: String,
}
