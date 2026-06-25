use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::{hash_password, CurrentUser};
use crate::errors::AppError;
use crate::models::{CreateUser, UpdateUser, User, UserView};

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route(
            "/users/:id",
            axum::routing::patch(update_user).delete(delete_user),
        )
}

fn require_admin(current: &CurrentUser) -> Result<(), AppError> {
    if current.role != "admin" {
        return Err(AppError::Forbidden("admin role required".into()));
    }
    Ok(())
}

fn validate_role(role: &str) -> Result<(), AppError> {
    if role != "admin" && role != "user" {
        return Err(AppError::BadRequest(
            "role must be 'admin' or 'user'".into(),
        ));
    }
    Ok(())
}

async fn list_users(
    State(pool): State<SqlitePool>,
    Extension(current): Extension<CurrentUser>,
) -> Result<Json<Vec<UserView>>, AppError> {
    require_admin(&current)?;

    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await?;
    Ok(Json(users.into_iter().map(UserView::from).collect()))
}

async fn create_user(
    State(pool): State<SqlitePool>,
    Extension(current): Extension<CurrentUser>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserView>), AppError> {
    require_admin(&current)?;

    let role = payload.role.unwrap_or_else(|| "user".to_string());
    validate_role(&role)?;

    let taken: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(&payload.username)
        .fetch_one(&pool)
        .await?;
    if taken > 0 {
        return Err(AppError::Conflict("username already taken".into()));
    }

    let id = Uuid::new_v4().to_string();
    let password_hash = hash_password(&payload.password)?;
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, totp_secret, role, created_at) VALUES (?, ?, ?, NULL, ?, ?)",
    )
    .bind(&id)
    .bind(&payload.username)
    .bind(&password_hash)
    .bind(&role)
    .bind(&created_at)
    .execute(&pool)
    .await?;

    let user = fetch_user(&pool, &id).await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

async fn update_user(
    State(pool): State<SqlitePool>,
    Extension(current): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserView>, AppError> {
    require_admin(&current)?;

    let existing = fetch_user(&pool, &id).await?;

    let username = payload.username.unwrap_or(existing.username);
    let role = payload.role.unwrap_or(existing.role);
    validate_role(&role)?;

    let password_hash = match payload.password {
        Some(password) => hash_password(&password)?,
        None => existing.password_hash,
    };

    sqlx::query("UPDATE users SET username = ?, password_hash = ?, role = ? WHERE id = ?")
        .bind(&username)
        .bind(&password_hash)
        .bind(&role)
        .bind(&id)
        .execute(&pool)
        .await?;

    let user = fetch_user(&pool, &id).await?;
    Ok(Json(user.into()))
}

async fn delete_user(
    State(pool): State<SqlitePool>,
    Extension(current): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    require_admin(&current)?;

    if current.id == id {
        return Err(AppError::BadRequest(
            "cannot delete your own account".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn fetch_user(pool: &SqlitePool, id: &str) -> Result<User, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}
