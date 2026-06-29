use sqlx::FromRow;

/// One row per OAuth grant in flight or active. Never exposed directly via
/// any API response — purely internal bookkeeping for the authorization code
/// + token exchange dance in `routes::oauth`.
#[derive(Debug, Clone, FromRow)]
pub struct OauthToken {
    pub id: String,
    pub user_id: String,
    pub code: Option<String>,
    pub code_expires_at: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub redirect_uri: Option<String>,
    pub access_token: Option<String>,
    pub access_expires_at: Option<String>,
    pub refresh_token: Option<String>,
    pub refresh_expires_at: Option<String>,
    pub client_id: String,
    pub created_at: String,
}
