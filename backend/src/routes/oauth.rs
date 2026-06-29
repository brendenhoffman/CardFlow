use axum::extract::{Form, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use url::Url;
use uuid::Uuid;

use crate::auth::{
    constant_time_eq, generate_refresh_token, jwt_secret, pkce_verify, verify_access_token,
    OAUTH_ACCESS_TOKEN_TTL_SECONDS, OAUTH_CODE_TTL_SECONDS, OAUTH_REFRESH_TOKEN_TTL_DAYS,
};
use crate::errors::AppError;
use crate::models::OauthToken;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/token", post(token))
}

fn oauth_client_id() -> Result<String, AppError> {
    std::env::var("OAUTH_CLIENT_ID")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::Internal("OAuth is not configured on this server".into()))
}

fn oauth_client_secret() -> Result<String, AppError> {
    std::env::var("OAUTH_CLIENT_SECRET")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::Internal("OAuth is not configured on this server".into()))
}

fn frontend_url() -> Result<String, AppError> {
    std::env::var("CARDFLOW_PUBLIC_URL")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::Internal("CARDFLOW_PUBLIC_URL is not configured".into()))
}

#[derive(Debug, Deserialize)]
struct AuthorizeQuery {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthorizeResponse {
    redirect_to: String,
}

/// Validates the authorize request, then either redirects an unauthenticated
/// browser to the Cardflow frontend's login page, or — when called with a
/// valid session Bearer token, which only happens via an authenticated fetch
/// from that login page after the user signs in — mints a code and reports
/// back the final redirect (to the OAuth client's own redirect_uri).
async fn authorize(
    State(pool): State<SqlitePool>,
    Query(params): Query<AuthorizeQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let configured_client_id = oauth_client_id()?;

    if params.response_type.as_deref() != Some("code") {
        return Err(AppError::BadRequest(
            "response_type must be \"code\"".into(),
        ));
    }
    let client_id = params.client_id.as_deref().unwrap_or_default();
    if client_id != configured_client_id {
        return Err(AppError::BadRequest("unknown client_id".into()));
    }
    let redirect_uri = params
        .redirect_uri
        .as_deref()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("redirect_uri is required".into()))?;
    Url::parse(redirect_uri)
        .map_err(|_| AppError::BadRequest("redirect_uri is not a valid URL".into()))?;
    let code_challenge = params
        .code_challenge
        .as_deref()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("code_challenge is required".into()))?;
    if params.code_challenge_method.as_deref() != Some("S256") {
        return Err(AppError::BadRequest(
            "code_challenge_method must be \"S256\"".into(),
        ));
    }

    let claims = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|token| verify_access_token(token, &jwt_secret().ok()?).ok());

    let Some(claims) = claims else {
        // Not authenticated yet -- send the browser to the frontend's login
        // page, carrying the original (now-validated) request through.
        let frontend = frontend_url()?;
        let mut location = Url::parse(&format!("{frontend}/oauth/authorize"))
            .map_err(|_| AppError::Internal("CARDFLOW_PUBLIC_URL is not a valid URL".into()))?;
        {
            let mut qp = location.query_pairs_mut();
            qp.append_pair("response_type", "code");
            qp.append_pair("client_id", client_id);
            qp.append_pair("redirect_uri", redirect_uri);
            qp.append_pair("code_challenge", code_challenge);
            qp.append_pair("code_challenge_method", "S256");
            if let Some(state) = &params.state {
                qp.append_pair("state", state);
            }
            if let Some(scope) = &params.scope {
                qp.append_pair("scope", scope);
            }
        }
        return Ok(Redirect::to(location.as_str()).into_response());
    };

    let id = Uuid::new_v4().to_string();
    let code = generate_refresh_token();
    let now = Utc::now();
    let code_expires_at = (now + Duration::seconds(OAUTH_CODE_TTL_SECONDS)).to_rfc3339();
    let created_at = now.to_rfc3339();

    sqlx::query(
        "INSERT INTO oauth_tokens \
            (id, user_id, code, code_expires_at, code_challenge, code_challenge_method, redirect_uri, client_id, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&code)
    .bind(&code_expires_at)
    .bind(code_challenge)
    .bind("S256")
    .bind(redirect_uri)
    .bind(client_id)
    .bind(&created_at)
    .execute(&pool)
    .await?;

    let mut final_redirect = Url::parse(redirect_uri).expect("validated above");
    {
        let mut qp = final_redirect.query_pairs_mut();
        qp.append_pair("code", &code);
        if let Some(state) = &params.state {
            qp.append_pair("state", state);
        }
    }

    Ok(Json(AuthorizeResponse {
        redirect_to: final_redirect.to_string(),
    })
    .into_response())
}

#[derive(Debug, Deserialize)]
struct TokenRequest {
    grant_type: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    redirect_uri: Option<String>,
    #[serde(default)]
    code_verifier: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    client_secret: Option<String>,
}

#[derive(Debug, Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: i64,
    refresh_token: String,
}

async fn token(
    State(pool): State<SqlitePool>,
    Form(req): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let configured_client_id = oauth_client_id()?;
    let configured_client_secret = oauth_client_secret()?;

    let client_id = req.client_id.as_deref().unwrap_or_default();
    let client_secret = req.client_secret.as_deref().unwrap_or_default();
    if !constant_time_eq(client_id, &configured_client_id)
        || !constant_time_eq(client_secret, &configured_client_secret)
    {
        return Err(AppError::Unauthorized("invalid client credentials".into()));
    }

    match req.grant_type.as_str() {
        "authorization_code" => exchange_code(&pool, &req).await,
        "refresh_token" => exchange_refresh(&pool, &req).await,
        _ => Err(AppError::BadRequest("unsupported grant_type".into())),
    }
}

async fn exchange_code(
    pool: &SqlitePool,
    req: &TokenRequest,
) -> Result<Json<TokenResponse>, AppError> {
    let code = req
        .code
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("code is required".into()))?;
    let redirect_uri = req
        .redirect_uri
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("redirect_uri is required".into()))?;
    let code_verifier = req
        .code_verifier
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("code_verifier is required".into()))?;

    let row = sqlx::query_as::<_, OauthToken>("SELECT * FROM oauth_tokens WHERE code = ?")
        .bind(code)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::BadRequest("invalid_grant: unknown code".into()))?;

    if row.client_id != req.client_id.as_deref().unwrap_or_default() {
        return Err(AppError::BadRequest(
            "invalid_grant: client_id mismatch".into(),
        ));
    }

    let now = Utc::now();
    let code_expires_at = row.code_expires_at.as_deref().unwrap_or_default();
    if code_expires_at < now.to_rfc3339().as_str() {
        // Best-effort cleanup of the expired, now-useless code.
        sqlx::query(
            "UPDATE oauth_tokens SET code = NULL, code_challenge = NULL, code_challenge_method = NULL, redirect_uri = NULL WHERE id = ?",
        )
        .bind(&row.id)
        .execute(pool)
        .await?;
        return Err(AppError::BadRequest("invalid_grant: code expired".into()));
    }

    if row.redirect_uri.as_deref() != Some(redirect_uri) {
        return Err(AppError::BadRequest(
            "invalid_grant: redirect_uri mismatch".into(),
        ));
    }

    if row.code_challenge_method.as_deref() != Some("S256") {
        // /authorize only ever stores "S256" (the only method this server
        // advertises), so this only trips if the row is malformed somehow --
        // pkce_verify assumes SHA-256, so check before relying on it.
        return Err(AppError::BadRequest(
            "invalid_grant: unsupported code_challenge_method".into(),
        ));
    }
    let challenge = row.code_challenge.as_deref().unwrap_or_default();
    if !pkce_verify(code_verifier, challenge) {
        return Err(AppError::BadRequest(
            "invalid_grant: PKCE verification failed".into(),
        ));
    }

    let access_token = generate_refresh_token();
    let refresh_token = generate_refresh_token();
    let access_expires_at = (now + Duration::seconds(OAUTH_ACCESS_TOKEN_TTL_SECONDS)).to_rfc3339();
    let refresh_expires_at = (now + Duration::days(OAUTH_REFRESH_TOKEN_TTL_DAYS)).to_rfc3339();

    sqlx::query(
        "UPDATE oauth_tokens SET \
            code = NULL, code_expires_at = NULL, code_challenge = NULL, code_challenge_method = NULL, redirect_uri = NULL, \
            access_token = ?, access_expires_at = ?, refresh_token = ?, refresh_expires_at = ? \
         WHERE id = ?",
    )
    .bind(&access_token)
    .bind(&access_expires_at)
    .bind(&refresh_token)
    .bind(&refresh_expires_at)
    .bind(&row.id)
    .execute(pool)
    .await?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: OAUTH_ACCESS_TOKEN_TTL_SECONDS,
        refresh_token,
    }))
}

async fn exchange_refresh(
    pool: &SqlitePool,
    req: &TokenRequest,
) -> Result<Json<TokenResponse>, AppError> {
    let refresh_token = req
        .refresh_token
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("refresh_token is required".into()))?;

    let row = sqlx::query_as::<_, OauthToken>("SELECT * FROM oauth_tokens WHERE refresh_token = ?")
        .bind(refresh_token)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::BadRequest("invalid_grant: unknown refresh_token".into()))?;

    if row.client_id != req.client_id.as_deref().unwrap_or_default() {
        return Err(AppError::BadRequest(
            "invalid_grant: client_id mismatch".into(),
        ));
    }

    let now = Utc::now();
    let refresh_expires_at = row.refresh_expires_at.as_deref().unwrap_or_default();
    if refresh_expires_at < now.to_rfc3339().as_str() {
        return Err(AppError::BadRequest(
            "invalid_grant: refresh_token expired".into(),
        ));
    }

    let access_token = generate_refresh_token();
    let new_refresh_token = generate_refresh_token();
    let access_expires_at = (now + Duration::seconds(OAUTH_ACCESS_TOKEN_TTL_SECONDS)).to_rfc3339();
    let new_refresh_expires_at = (now + Duration::days(OAUTH_REFRESH_TOKEN_TTL_DAYS)).to_rfc3339();

    sqlx::query(
        "UPDATE oauth_tokens SET access_token = ?, access_expires_at = ?, refresh_token = ?, refresh_expires_at = ? WHERE id = ?",
    )
    .bind(&access_token)
    .bind(&access_expires_at)
    .bind(&new_refresh_token)
    .bind(&new_refresh_expires_at)
    .bind(&row.id)
    .execute(pool)
    .await?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: OAUTH_ACCESS_TOKEN_TTL_SECONDS,
        refresh_token: new_refresh_token,
    }))
}
