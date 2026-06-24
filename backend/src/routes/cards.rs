use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{Card, CardJoker, CreateCard, CreateJoker, Stack, UpdateCard};
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
        .route("/cards/:id/stack", get(get_stack))
        .route("/cards/:id/jokers", post(create_joker).get(list_jokers))
        .route(
            "/cards/:id/jokers/:joker_id",
            axum::routing::delete(delete_joker),
        )
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

#[derive(Debug, serde::Serialize)]
struct CompleteResult {
    card: Card,
    unblocked: Vec<Card>,
}

async fn complete_card(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<CompleteResult>, AppError> {
    let mut tx = pool.begin().await?;

    let existing = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ?")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;

    if existing.status != "hand" {
        return Err(AppError::BadRequest(
            "only cards in hand can be completed".into(),
        ));
    }

    if is_blocked(&mut tx, &id).await? {
        return Err(AppError::BadRequest(
            "card is blocked by incomplete jokers".into(),
        ));
    }

    let completed_at = Utc::now().to_rfc3339();

    sqlx::query("UPDATE cards SET status = 'done', priority = NULL, completed_at = ? WHERE id = ?")
        .bind(&completed_at)
        .bind(&id)
        .execute(&mut *tx)
        .await?;

    let parent_ids: Vec<String> =
        sqlx::query_scalar("SELECT card_id FROM card_jokers WHERE joker_id = ?")
            .bind(&id)
            .fetch_all(&mut *tx)
            .await?;

    let mut unblocked = Vec::new();
    for parent_id in parent_ids {
        let parent = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ?")
            .bind(&parent_id)
            .fetch_one(&mut *tx)
            .await?;
        if parent.status == "hand" && !is_blocked(&mut tx, &parent_id).await? {
            unblocked.push(parent);
        }
    }

    let card = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ?")
        .bind(&id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(CompleteResult { card, unblocked }))
}

async fn return_card(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Card>>, AppError> {
    let existing = fetch_card(&pool, &id).await?;

    if existing.status != "hand" || existing.priority.is_none() {
        return Err(AppError::BadRequest(
            "only the root of a stack in hand can be returned to the pile".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    let stack_cards = fetch_tree_cards(&mut tx, &id).await?;
    let mut returned = Vec::new();
    for mut card in stack_cards {
        if card.status == "hand" {
            sqlx::query("UPDATE cards SET status = 'pile', priority = NULL WHERE id = ?")
                .bind(&card.id)
                .execute(&mut *tx)
                .await?;
            card.status = "pile".to_string();
            card.priority = None;
            returned.push(card);
        }
    }

    tx.commit().await?;

    Ok(Json(returned))
}

async fn get_stack(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<Stack>, AppError> {
    fetch_card(&pool, &id).await?;
    let mut conn = pool.acquire().await?;
    let stack = fetch_stack(&mut conn, &id).await?;
    Ok(Json(stack))
}

async fn list_jokers(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Card>>, AppError> {
    fetch_card(&pool, &id).await?;

    let jokers = sqlx::query_as::<_, Card>(
        r#"SELECT c.* FROM card_jokers cj
           JOIN cards c ON c.id = cj.joker_id
           WHERE cj.card_id = ?
           ORDER BY cj."order" ASC"#,
    )
    .bind(&id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(jokers))
}

async fn create_joker(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<CreateJoker>,
) -> Result<(StatusCode, Json<CardJoker>), AppError> {
    let card = fetch_card(&pool, &id).await?;
    let joker = fetch_card(&pool, &payload.joker_id).await?;

    if card.id == joker.id {
        return Err(AppError::BadRequest(
            "a card cannot depend on itself".into(),
        ));
    }
    if card.deck_id != joker.deck_id {
        return Err(AppError::BadRequest(
            "a joker must belong to the same deck as the card".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    let ancestor_ids: Vec<String> = fetch_tree_cards(&mut tx, &joker.id)
        .await?
        .into_iter()
        .map(|c| c.id)
        .collect();
    if ancestor_ids.contains(&card.id) {
        return Err(AppError::BadRequest(
            "adding this joker would create a dependency cycle".into(),
        ));
    }

    let next_order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(\"order\"), 0) + 1 FROM card_jokers WHERE card_id = ?",
    )
    .bind(&id)
    .fetch_one(&mut *tx)
    .await?;

    let joker_edge_id = Uuid::new_v4().to_string();
    sqlx::query(r#"INSERT INTO card_jokers (id, card_id, joker_id, "order") VALUES (?, ?, ?, ?)"#)
        .bind(&joker_edge_id)
        .bind(&id)
        .bind(&payload.joker_id)
        .bind(next_order)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let edge = sqlx::query_as::<_, CardJoker>("SELECT * FROM card_jokers WHERE id = ?")
        .bind(&joker_edge_id)
        .fetch_one(&pool)
        .await?;

    Ok((StatusCode::CREATED, Json(edge)))
}

async fn delete_joker(
    State(pool): State<SqlitePool>,
    Path((id, joker_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM card_jokers WHERE card_id = ? AND joker_id = ?")
        .bind(&id)
        .bind(&joker_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Whether `card_id` has any joker dependency that isn't done yet.
pub(crate) async fn is_blocked(
    conn: &mut SqliteConnection,
    card_id: &str,
) -> Result<bool, AppError> {
    let blocked: Option<i64> = sqlx::query_scalar(
        r#"SELECT 1 FROM card_jokers cj
           JOIN cards j ON j.id = cj.joker_id
           WHERE cj.card_id = ? AND j.status != 'done'
           LIMIT 1"#,
    )
    .bind(card_id)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(blocked.is_some())
}

/// Flat list of every card in `root_id`'s joker subtree (any status), via a recursive CTE.
pub(crate) async fn fetch_tree_cards(
    conn: &mut SqliteConnection,
    root_id: &str,
) -> Result<Vec<Card>, AppError> {
    let cards = sqlx::query_as::<_, Card>(
        r#"WITH RECURSIVE tree(card_id) AS (
               SELECT ?1
               UNION
               SELECT cj.joker_id FROM card_jokers cj JOIN tree t ON cj.card_id = t.card_id
           )
           SELECT c.* FROM cards c JOIN tree t ON c.id = t.card_id"#,
    )
    .bind(root_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(cards)
}

/// Nested {card, jokers} tree for `root_id`, for API responses.
pub(crate) async fn fetch_stack(
    conn: &mut SqliteConnection,
    root_id: &str,
) -> Result<Stack, AppError> {
    let cards = fetch_tree_cards(conn, root_id).await?;
    let edges: Vec<(String, String)> = sqlx::query_as(
        r#"WITH RECURSIVE tree(card_id) AS (
               SELECT ?1
               UNION
               SELECT cj.joker_id FROM card_jokers cj JOIN tree t ON cj.card_id = t.card_id
           )
           SELECT cj.card_id, cj.joker_id
           FROM card_jokers cj
           JOIN tree t ON cj.card_id = t.card_id
           ORDER BY cj.card_id, cj."order""#,
    )
    .bind(root_id)
    .fetch_all(&mut *conn)
    .await?;

    let cards_by_id: HashMap<String, Card> = cards.into_iter().map(|c| (c.id.clone(), c)).collect();
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for (card_id, joker_id) in edges {
        children.entry(card_id).or_default().push(joker_id);
    }

    fn build(
        id: &str,
        cards: &HashMap<String, Card>,
        children: &HashMap<String, Vec<String>>,
    ) -> Stack {
        let card = cards
            .get(id)
            .cloned()
            .expect("card present in tree result set");
        let jokers = children
            .get(id)
            .into_iter()
            .flatten()
            .map(|child_id| build(child_id, cards, children))
            .collect();
        Stack { card, jokers }
    }

    Ok(build(root_id, &cards_by_id, &children))
}
