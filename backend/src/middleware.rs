use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;
use sqlx::SqlitePool;

use crate::auth::{jwt_secret, verify_access_token, verify_password, CurrentUser};
use crate::errors::AppError;
use crate::models::ApiToken;

pub async fn require_auth(
    State(pool): State<SqlitePool>,
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing authorization header".into()))?;

    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        AppError::Unauthorized("authorization header must be a bearer token".into())
    })?;

    // JWTs and API tokens share the same `Authorization: Bearer <value>` header.
    // A short-lived session JWT is the common case, so try that first; anything
    // that isn't a validly-signed JWT (including long-lived API tokens, which
    // are opaque random strings) falls through to the API token check.
    let current = match authenticate_jwt(&pool, token).await {
        Ok(current) => current,
        Err(_) => authenticate_api_token(&pool, token).await?,
    };

    req.extensions_mut().insert(current);

    Ok(next.run(req).await)
}

async fn authenticate_jwt(pool: &SqlitePool, token: &str) -> Result<CurrentUser, AppError> {
    let secret = jwt_secret()?;
    let claims = verify_access_token(token, &secret)?;

    let row =
        sqlx::query_as::<_, (String, String)>("SELECT username, role FROM users WHERE id = ?")
            .bind(&claims.sub)
            .fetch_optional(pool)
            .await?;
    let (username, role) =
        row.ok_or_else(|| AppError::Unauthorized("user no longer exists".into()))?;

    Ok(CurrentUser {
        id: claims.sub,
        username,
        role,
    })
}

/// API tokens are hashed with argon2, which is salted and non-deterministic —
/// unlike the refresh token (looked up by an exact SHA-256 hash match) we can't
/// index straight to a row, so every stored token is scanned and verified
/// against the candidate. Fine at this app's expected scale (self-hosted,
/// a handful of users/tokens); updates last_used_at on a successful match.
async fn authenticate_api_token(pool: &SqlitePool, token: &str) -> Result<CurrentUser, AppError> {
    let candidates = sqlx::query_as::<_, ApiToken>("SELECT * FROM api_tokens")
        .fetch_all(pool)
        .await?;

    let matched = candidates
        .into_iter()
        .find(|candidate| verify_password(token, &candidate.token_hash).unwrap_or(false))
        .ok_or_else(|| AppError::Unauthorized("invalid bearer token".into()))?;

    let row =
        sqlx::query_as::<_, (String, String)>("SELECT username, role FROM users WHERE id = ?")
            .bind(&matched.user_id)
            .fetch_optional(pool)
            .await?;
    let (username, role) =
        row.ok_or_else(|| AppError::Unauthorized("user no longer exists".into()))?;

    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE api_tokens SET last_used_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&matched.id)
        .execute(pool)
        .await?;

    Ok(CurrentUser {
        id: matched.user_id,
        username,
        role,
    })
}
