use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{Card, CreateCard, UpdateCard};
use crate::routes::decks::fetch_deck;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/decks/:deck_id/cards", get(list_cards).post(create_card))
        .route(
            "/cards/:id",
            get(get_card).patch(update_card).delete(delete_card),
        )
        .route("/cards/:id/complete", post(complete_card))
        .route("/cards/:id/return", post(return_card))
}

async fn list_cards(
    State(pool): State<SqlitePool>,
    Path(deck_id): Path<String>,
) -> Result<Json<Vec<Card>>, AppError> {
    fetch_deck(&pool, &deck_id).await?;

    let cards =
        sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE deck_id = ? ORDER BY created_at DESC")
            .bind(&deck_id)
            .fetch_all(&pool)
            .await?;
    Ok(Json(cards))
}

async fn get_card(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<Card>, AppError> {
    let card = fetch_card(&pool, &id).await?;
    Ok(Json(card))
}

async fn create_card(
    State(pool): State<SqlitePool>,
    Path(deck_id): Path<String>,
    Json(payload): Json<CreateCard>,
) -> Result<(StatusCode, Json<Card>), AppError> {
    fetch_deck(&pool, &deck_id).await?;

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO cards (id, deck_id, title, description, status, priority, created_at, completed_at) \
         VALUES (?, ?, ?, ?, 'pile', NULL, ?, NULL)",
    )
    .bind(&id)
    .bind(&deck_id)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&created_at)
    .execute(&pool)
    .await?;

    let card = fetch_card(&pool, &id).await?;
    Ok((StatusCode::CREATED, Json(card)))
}

async fn update_card(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateCard>,
) -> Result<Json<Card>, AppError> {
    let existing = fetch_card(&pool, &id).await?;

    let title = payload.title.unwrap_or(existing.title);
    let description = payload.description.or(existing.description);
    let status = payload.status.unwrap_or(existing.status);
    let priority = payload.priority.or(existing.priority);

    sqlx::query(
        "UPDATE cards SET title = ?, description = ?, status = ?, priority = ? WHERE id = ?",
    )
    .bind(&title)
    .bind(&description)
    .bind(&status)
    .bind(priority)
    .bind(&id)
    .execute(&pool)
    .await?;

    let card = fetch_card(&pool, &id).await?;
    Ok(Json(card))
}

async fn delete_card(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM cards WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn fetch_card(pool: &SqlitePool, id: &str) -> Result<Card, AppError> {
    sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn complete_card(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<Card>, AppError> {
    let existing = fetch_card(&pool, &id).await?;

    if existing.status != "hand" {
        return Err(AppError::BadRequest(
            "only cards in hand can be completed".into(),
        ));
    }

    let completed_at = Utc::now().to_rfc3339();

    sqlx::query("UPDATE cards SET status = 'done', priority = NULL, completed_at = ? WHERE id = ?")
        .bind(&completed_at)
        .bind(&id)
        .execute(&pool)
        .await?;

    let card = fetch_card(&pool, &id).await?;
    Ok(Json(card))
}

async fn return_card(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<Card>, AppError> {
    let existing = fetch_card(&pool, &id).await?;

    if existing.status != "hand" {
        return Err(AppError::BadRequest(
            "only cards in hand can be returned to the pile".into(),
        ));
    }

    sqlx::query("UPDATE cards SET status = 'pile', priority = NULL WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;

    let card = fetch_card(&pool, &id).await?;
    Ok(Json(card))
}
