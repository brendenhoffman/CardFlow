use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::{generate_api_token, hash_password, CurrentUser};
use crate::errors::AppError;
use crate::models::{ApiToken, ApiTokenView, CreateApiToken, CreateApiTokenResponse};

pub fn router() -> Router<SqlitePool> {
    Router::new().route(
        "/api-tokens",
        get(list_api_tokens).post(create_api_token),
    )
    .route("/api-tokens/:id", axum::routing::delete(delete_api_token))
}

async fn list_api_tokens(
    State(pool): State<SqlitePool>,
    Extension(current): Extension<CurrentUser>,
) -> Result<Json<Vec<ApiTokenView>>, AppError> {
    let tokens = sqlx::query_as::<_, ApiToken>(
        "SELECT * FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&current.id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(tokens.into_iter().map(ApiTokenView::from).collect()))
}

async fn create_api_token(
    State(pool): State<SqlitePool>,
    Extension(current): Extension<CurrentUser>,
    Json(payload): Json<CreateApiToken>,
) -> Result<(StatusCode, Json<CreateApiTokenResponse>), AppError> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let id = Uuid::new_v4().to_string();
    let raw_token = generate_api_token();
    let token_hash = hash_password(&raw_token)?;
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, created_at, last_used_at) VALUES (?, ?, ?, ?, ?, NULL)",
    )
    .bind(&id)
    .bind(&current.id)
    .bind(&name)
    .bind(&token_hash)
    .bind(&created_at)
    .execute(&pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateApiTokenResponse {
            id,
            name,
            token: raw_token,
            created_at,
        }),
    ))
}

async fn delete_api_token(
    State(pool): State<SqlitePool>,
    Extension(current): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM api_tokens WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&current.id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}
