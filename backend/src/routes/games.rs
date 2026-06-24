use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{CreateGame, Game, UpdateGame};

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/games", get(list_games).post(create_game))
        .route(
            "/games/:id",
            get(get_game).patch(update_game).delete(delete_game),
        )
}

async fn list_games(State(pool): State<SqlitePool>) -> Result<Json<Vec<Game>>, AppError> {
    let games = sqlx::query_as::<_, Game>("SELECT * FROM games ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await?;
    Ok(Json(games))
}

async fn get_game(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<Game>, AppError> {
    let game = fetch_game(&pool, &id).await?;
    Ok(Json(game))
}

async fn create_game(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateGame>,
) -> Result<(StatusCode, Json<Game>), AppError> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO games (id, name, description, status, created_at) VALUES (?, ?, ?, 'active', ?)",
    )
    .bind(&id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&created_at)
    .execute(&pool)
    .await?;

    let game = fetch_game(&pool, &id).await?;
    Ok((StatusCode::CREATED, Json(game)))
}

async fn update_game(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateGame>,
) -> Result<Json<Game>, AppError> {
    let existing = fetch_game(&pool, &id).await?;

    let name = payload.name.unwrap_or(existing.name);
    let description = payload.description.or(existing.description);
    let status = payload.status.unwrap_or(existing.status);

    sqlx::query("UPDATE games SET name = ?, description = ?, status = ? WHERE id = ?")
        .bind(&name)
        .bind(&description)
        .bind(&status)
        .bind(&id)
        .execute(&pool)
        .await?;

    let game = fetch_game(&pool, &id).await?;
    Ok(Json(game))
}

async fn delete_game(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM games WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn fetch_game(pool: &SqlitePool, id: &str) -> Result<Game, AppError> {
    sqlx::query_as::<_, Game>("SELECT * FROM games WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}
