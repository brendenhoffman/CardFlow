use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::models::Card;

#[derive(Debug, Serialize, FromRow)]
pub struct CardJoker {
    pub id: String,
    pub card_id: String,
    pub joker_id: String,
    pub order: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateJoker {
    pub joker_id: String,
}

/// Nested view of a card and its joker subtree, for collapsed/expanded UI rendering.
#[derive(Debug, Serialize)]
pub struct Stack {
    pub card: Card,
    pub jokers: Vec<Stack>,
}
