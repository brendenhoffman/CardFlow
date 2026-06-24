use std::collections::HashSet;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{CreateDeck, Deck, Stack, UpdateDeck};
use crate::routes::cards::fetch_stack;
use crate::routes::games::fetch_game;

const MAX_HAND_SIZE: usize = 5;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/games/:game_id/decks", get(list_decks).post(create_deck))
        .route(
            "/decks/:id",
            get(get_deck).patch(update_deck).delete(delete_deck),
        )
        .route("/decks/:deck_id/deal", post(deal_hand))
        .route("/decks/:deck_id/draw/:card_id", post(draw_card))
        .route(
            "/decks/:deck_id/reorder",
            axum::routing::patch(reorder_hand),
        )
}

async fn list_decks(
    State(pool): State<SqlitePool>,
    Path(game_id): Path<String>,
) -> Result<Json<Vec<Deck>>, AppError> {
    fetch_game(&pool, &game_id).await?;

    let decks =
        sqlx::query_as::<_, Deck>("SELECT * FROM decks WHERE game_id = ? ORDER BY created_at DESC")
            .bind(&game_id)
            .fetch_all(&pool)
            .await?;
    Ok(Json(decks))
}

async fn get_deck(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<Deck>, AppError> {
    let deck = fetch_deck(&pool, &id).await?;
    Ok(Json(deck))
}

async fn create_deck(
    State(pool): State<SqlitePool>,
    Path(game_id): Path<String>,
    Json(payload): Json<CreateDeck>,
) -> Result<(StatusCode, Json<Deck>), AppError> {
    fetch_game(&pool, &game_id).await?;

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO decks (id, game_id, name, description, status, created_at) VALUES (?, ?, ?, ?, 'active', ?)",
    )
    .bind(&id)
    .bind(&game_id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&created_at)
    .execute(&pool)
    .await?;

    let deck = fetch_deck(&pool, &id).await?;
    Ok((StatusCode::CREATED, Json(deck)))
}

async fn update_deck(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDeck>,
) -> Result<Json<Deck>, AppError> {
    let existing = fetch_deck(&pool, &id).await?;

    let name = payload.name.unwrap_or(existing.name);
    let description = payload.description.or(existing.description);
    let status = payload.status.unwrap_or(existing.status);

    sqlx::query("UPDATE decks SET name = ?, description = ?, status = ? WHERE id = ?")
        .bind(&name)
        .bind(&description)
        .bind(&status)
        .bind(&id)
        .execute(&pool)
        .await?;

    let deck = fetch_deck(&pool, &id).await?;
    Ok(Json(deck))
}

async fn delete_deck(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM decks WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn fetch_deck(pool: &SqlitePool, id: &str) -> Result<Deck, AppError> {
    sqlx::query_as::<_, Deck>("SELECT * FROM decks WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

#[derive(Debug, Deserialize)]
struct ReorderRequest {
    order: Vec<String>,
}

async fn deal_hand(
    State(pool): State<SqlitePool>,
    Path(deck_id): Path<String>,
) -> Result<Json<Vec<Stack>>, AppError> {
    fetch_deck(&pool, &deck_id).await?;

    let mut tx = pool.begin().await?;

    loop {
        let used = hand_priorities(&mut tx, &deck_id).await?;
        if used.len() >= MAX_HAND_SIZE {
            break;
        }

        let candidate: Option<String> = sqlx::query_scalar(
            "SELECT id FROM cards WHERE deck_id = ? AND status = 'pile' ORDER BY RANDOM() LIMIT 1",
        )
        .bind(&deck_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(candidate_id) = candidate else {
            break;
        };

        let priority = available_priorities(&used, 1)
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Conflict("hand is full".into()))?;

        draw_stack_into_hand(&mut tx, &candidate_id, priority).await?;
    }

    tx.commit().await?;

    Ok(Json(fetch_hand(&pool, &deck_id).await?))
}

async fn draw_card(
    State(pool): State<SqlitePool>,
    Path((deck_id, card_id)): Path<(String, String)>,
) -> Result<Json<Stack>, AppError> {
    fetch_deck(&pool, &deck_id).await?;

    let mut tx = pool.begin().await?;

    let card_deck_id: Option<String> = sqlx::query_scalar("SELECT deck_id FROM cards WHERE id = ?")
        .bind(&card_id)
        .fetch_optional(&mut *tx)
        .await?;
    if card_deck_id.as_deref() != Some(deck_id.as_str()) {
        return Err(AppError::NotFound);
    }

    let card_status: String = sqlx::query_scalar("SELECT status FROM cards WHERE id = ?")
        .bind(&card_id)
        .fetch_one(&mut *tx)
        .await?;
    if card_status != "pile" {
        return Err(AppError::BadRequest("card is not in the pile".into()));
    }

    let used = hand_priorities(&mut tx, &deck_id).await?;
    let priority = available_priorities(&used, 1)
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Conflict("hand is full".into()))?;

    draw_stack_into_hand(&mut tx, &card_id, priority).await?;

    let stack = fetch_stack(&mut tx, &card_id).await?;
    tx.commit().await?;

    Ok(Json(stack))
}

/// Walks `root_id`'s joker subtree and pulls every unresolved (not-done) node into
/// the hand as a single stack: the root gets `priority`, the rest get none.
async fn draw_stack_into_hand(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    root_id: &str,
    priority: i64,
) -> Result<(), AppError> {
    let nodes = crate::routes::cards::fetch_tree_cards(tx, root_id).await?;
    for node in nodes {
        if node.status == "done" {
            continue;
        }
        let node_priority = if node.id == root_id {
            Some(priority)
        } else {
            None
        };
        sqlx::query("UPDATE cards SET status = 'hand', priority = ? WHERE id = ?")
            .bind(node_priority)
            .bind(&node.id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn reorder_hand(
    State(pool): State<SqlitePool>,
    Path(deck_id): Path<String>,
    Json(payload): Json<ReorderRequest>,
) -> Result<Json<Vec<Stack>>, AppError> {
    fetch_deck(&pool, &deck_id).await?;

    if payload.order.is_empty() || payload.order.len() > MAX_HAND_SIZE {
        return Err(AppError::BadRequest(format!(
            "order must contain between 1 and {MAX_HAND_SIZE} card ids"
        )));
    }

    let requested: HashSet<&String> = payload.order.iter().collect();
    if requested.len() != payload.order.len() {
        return Err(AppError::BadRequest(
            "order contains duplicate card ids".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    // Only stack roots occupy a slot; joker stack members share the root's slot.
    let hand_ids: HashSet<String> = sqlx::query_scalar(
        "SELECT id FROM cards WHERE deck_id = ? AND status = 'hand' AND priority IS NOT NULL",
    )
    .bind(&deck_id)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();

    let requested_ids: HashSet<String> = payload.order.iter().cloned().collect();
    if requested_ids != hand_ids {
        return Err(AppError::BadRequest(
            "order must contain exactly the deck's current hand".into(),
        ));
    }

    for (index, card_id) in payload.order.iter().enumerate() {
        sqlx::query("UPDATE cards SET priority = ? WHERE id = ? AND deck_id = ?")
            .bind((index + 1) as i64)
            .bind(card_id)
            .bind(&deck_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(Json(fetch_hand(&pool, &deck_id).await?))
}

async fn hand_priorities(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    deck_id: &str,
) -> Result<Vec<i64>, AppError> {
    let priorities = sqlx::query_scalar(
        "SELECT priority FROM cards WHERE deck_id = ? AND status = 'hand' AND priority IS NOT NULL",
    )
    .bind(deck_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(priorities)
}

fn available_priorities(used: &[i64], needed: usize) -> Vec<i64> {
    (1..=MAX_HAND_SIZE as i64)
        .filter(|p| !used.contains(p))
        .take(needed)
        .collect()
}

async fn fetch_hand(pool: &SqlitePool, deck_id: &str) -> Result<Vec<Stack>, AppError> {
    let root_ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM cards WHERE deck_id = ? AND status = 'hand' AND priority IS NOT NULL ORDER BY priority ASC",
    )
    .bind(deck_id)
    .fetch_all(pool)
    .await?;

    let mut conn = pool.acquire().await?;
    let mut hand = Vec::with_capacity(root_ids.len());
    for root_id in root_ids {
        hand.push(fetch_stack(&mut conn, &root_id).await?);
    }
    Ok(hand)
}
