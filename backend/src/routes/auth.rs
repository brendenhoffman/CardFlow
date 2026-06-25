use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::{
    check_totp, cookie_secure, create_access_token, generate_refresh_token, generate_totp_secret,
    hash_token, jwt_secret, verify_password, CurrentUser, ACCESS_TOKEN_TTL_SECONDS,
    REFRESH_TOKEN_TTL_DAYS,
};
use crate::errors::AppError;
use crate::models::User;
use crate::routes::users::fetch_user;

const REFRESH_COOKIE_NAME: &str = "refresh_token";

pub fn public_router() -> Router<SqlitePool> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
}

pub fn protected_router() -> Router<SqlitePool> {
    Router::new()
        .route("/auth/mfa/setup", post(mfa_setup))
        .route("/auth/mfa/verify", post(mfa_verify))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
    #[serde(default)]
    totp_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: i64,
}

async fn login(
    State(pool): State<SqlitePool>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<SessionResponse>), AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(&payload.username)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("invalid username or password".into()))?;

    if !verify_password(&payload.password, &user.password_hash)? {
        return Err(AppError::Unauthorized(
            "invalid username or password".into(),
        ));
    }

    if let Some(secret) = &user.totp_secret {
        let code = payload
            .totp_code
            .as_deref()
            .ok_or_else(|| AppError::Unauthorized("totp code required".into()))?;
        if !check_totp(secret, &user.username, code)? {
            return Err(AppError::Unauthorized("invalid totp code".into()));
        }
    }

    let (jar, response) = issue_session(&pool, &user, jar).await?;
    Ok((jar, Json(response)))
}

async fn refresh(
    State(pool): State<SqlitePool>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<SessionResponse>), AppError> {
    let token = jar
        .get(REFRESH_COOKIE_NAME)
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::Unauthorized("missing refresh token".into()))?;

    let token_hash = hash_token(&token);
    let now = Utc::now().to_rfc3339();

    let row = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT user_id, expires_at, revoked_at FROM refresh_tokens WHERE token_hash = ?",
    )
    .bind(&token_hash)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("invalid refresh token".into()))?;

    let (user_id, expires_at, revoked_at) = row;
    if revoked_at.is_some() || expires_at < now {
        return Err(AppError::Unauthorized(
            "refresh token expired or revoked".into(),
        ));
    }

    sqlx::query("UPDATE refresh_tokens SET revoked_at = ? WHERE token_hash = ?")
        .bind(&now)
        .bind(&token_hash)
        .execute(&pool)
        .await?;

    let user = fetch_user(&pool, &user_id).await?;
    let (jar, response) = issue_session(&pool, &user, jar).await?;
    Ok((jar, Json(response)))
}

async fn logout(
    State(pool): State<SqlitePool>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), AppError> {
    if let Some(cookie) = jar.get(REFRESH_COOKIE_NAME) {
        let token_hash = hash_token(cookie.value());
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = ? WHERE token_hash = ? AND revoked_at IS NULL",
        )
        .bind(&now)
        .bind(&token_hash)
        .execute(&pool)
        .await?;
    }

    let removal = Cookie::build((REFRESH_COOKIE_NAME, ""))
        .path("/auth")
        .build();
    let jar = jar.remove(removal);
    Ok((jar, StatusCode::NO_CONTENT))
}

async fn issue_session(
    pool: &SqlitePool,
    user: &User,
    jar: CookieJar,
) -> Result<(CookieJar, SessionResponse), AppError> {
    let secret = jwt_secret()?;
    let access_token = create_access_token(user, &secret)?;

    let refresh_token = generate_refresh_token();
    let token_hash = hash_token(&refresh_token);
    let now = Utc::now();
    let expires_at = now + chrono::Duration::days(REFRESH_TOKEN_TTL_DAYS);

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at, revoked_at) VALUES (?, ?, ?, ?, ?, NULL)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user.id)
    .bind(&token_hash)
    .bind(expires_at.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;

    let cookie = Cookie::build((REFRESH_COOKIE_NAME, refresh_token))
        .http_only(true)
        .secure(cookie_secure())
        .same_site(SameSite::Strict)
        .path("/auth")
        .max_age(time::Duration::days(REFRESH_TOKEN_TTL_DAYS))
        .build();

    let jar = jar.add(cookie);
    let response = SessionResponse {
        access_token,
        token_type: "Bearer",
        expires_in: ACCESS_TOKEN_TTL_SECONDS,
    };
    Ok((jar, response))
}

#[derive(Debug, Serialize)]
struct MfaSetupResponse {
    secret: String,
    otpauth_url: String,
}

async fn mfa_setup(
    Extension(current): Extension<CurrentUser>,
) -> Result<Json<MfaSetupResponse>, AppError> {
    let (secret, otpauth_url) = generate_totp_secret(&current.username)?;
    Ok(Json(MfaSetupResponse {
        secret,
        otpauth_url,
    }))
}

#[derive(Debug, Deserialize)]
struct MfaVerifyRequest {
    secret: String,
    code: String,
}

async fn mfa_verify(
    State(pool): State<SqlitePool>,
    Extension(current): Extension<CurrentUser>,
    Json(payload): Json<MfaVerifyRequest>,
) -> Result<StatusCode, AppError> {
    if !check_totp(&payload.secret, &current.username, &payload.code)? {
        return Err(AppError::BadRequest("invalid totp code".into()));
    }

    sqlx::query("UPDATE users SET totp_secret = ? WHERE id = ?")
        .bind(&payload.secret)
        .bind(&current.id)
        .execute(&pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
