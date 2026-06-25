use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::hash_password;
use crate::errors::AppError;
use crate::models::UserView;
use crate::routes::users::fetch_user;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/setup/status", get(setup_status))
        .route("/setup", post(setup))
}

#[derive(Debug, Serialize)]
struct SetupStatus {
    required: bool,
}

async fn setup_status(State(pool): State<SqlitePool>) -> Result<Json<SetupStatus>, AppError> {
    let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(&pool)
        .await?;
    Ok(Json(SetupStatus {
        required: admin_count == 0,
    }))
}

#[derive(Debug, Deserialize)]
struct SetupRequest {
    username: String,
    password: String,
}

async fn setup(
    State(pool): State<SqlitePool>,
    Json(payload): Json<SetupRequest>,
) -> Result<(StatusCode, Json<UserView>), AppError> {
    if payload.username.trim().is_empty() {
        return Err(AppError::BadRequest("username is required".into()));
    }
    if payload.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&mut *tx)
        .await?;
    if user_count > 0 {
        return Err(AppError::Forbidden(
            "setup has already been completed".into(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let password_hash = hash_password(&payload.password)?;
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, totp_secret, role, created_at) VALUES (?, ?, ?, NULL, 'admin', ?)",
    )
    .bind(&id)
    .bind(&payload.username)
    .bind(&password_hash)
    .bind(&created_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let user = fetch_user(&pool, &id).await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}
