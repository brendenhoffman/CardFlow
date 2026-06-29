use sqlx::FromRow;

/// One row per OAuth grant in flight or active. Never exposed directly via
/// any API response — purely internal bookkeeping for the authorization code
/// + token exchange dance in `routes::oauth`.
#[derive(Debug, Clone, FromRow)]
pub struct OauthToken {
    pub id: String,
    pub user_id: String,
    // `code`, `access_token`, and `refresh_token` are only ever used as SQL
    // lookup keys (`WHERE code = ?`, etc.) or write targets -- once a row is
    // fetched by one of them, there's nothing to re-read it for in Rust.
    // `created_at` is audit-only. FromRow still needs the fields declared to
    // match `SELECT *`.
    #[allow(dead_code)]
    pub code: Option<String>,
    pub code_expires_at: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub redirect_uri: Option<String>,
    #[allow(dead_code)]
    pub access_token: Option<String>,
    pub access_expires_at: Option<String>,
    #[allow(dead_code)]
    pub refresh_token: Option<String>,
    pub refresh_expires_at: Option<String>,
    pub client_id: String,
    #[allow(dead_code)]
    pub created_at: String,
}
