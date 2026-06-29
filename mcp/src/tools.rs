use rmcp::handler::server::common::{AsRequestContext, FromContextPart};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use serde_json::Value;

use crate::cardflow_client::CardflowClient;

const INSTRUCTIONS: &str = r#"You are working with Cardflow, a project-management tool built around a card-game metaphor.

Mental model:
- Game = project. The top-level container; everything else lives under one.
- Deck = epic/sprint. A themed body of work within a game. Each deck has its own pile (backlog) and hand (up to 5 actively-worked cards, ranked by priority 1-5).
- Card = user story. A single unit of work, living in a deck's pile, hand, or done.
- Joker = a blocking dependency between two cards. If card A has joker B, A cannot be completed until B is done. Jokers can themselves have jokers, forming a dependency chain of any depth.

Rules to follow:
1. Only add a joker when a genuine blocking relationship exists -- B must literally need to happen before A can start or finish. Do not use jokers for organization, priority, or grouping; titles, descriptions, and decks already cover that.
2. Always call list_cards for the relevant deck before creating new cards, so you don't create duplicates of work that already exists.
3. When setting up a batch of related work, create all the cards first (so every card_id you need already exists), and only then call add_joker to wire dependencies between them.
4. Card titles should be short and actionable (e.g. "Add input validation to signup form"). Descriptions should carry enough technical context that a developer could pick up the card and implement it without asking a follow-up question -- mention relevant files, functions, edge cases, or constraints when you know them.
5. Prefer creating immediate, actionable cards over vague, big-picture ones. Only create broad/aspirational cards if the user explicitly asks for big-picture planning; default to concrete next steps.
"#;

/// Extracts the connecting client's own `Authorization: Bearer <token>` header
/// (an API token or an OAuth access token), if any, for use as this tool
/// call's Cardflow credentials. `rmcp`'s SSE transport attaches the original
/// HTTP request `Parts` for each JSON-RPC message as a context extension
/// (see `post_event_handler` in `rmcp::transport::sse_server`); this just
/// reads it back out. Never fails -- absence is valid, since `CardflowClient`
/// falls back to the server's static `CARDFLOW_TOKEN`, if configured.
pub struct BearerToken(pub Option<String>);

impl<C> FromContextPart<C> for BearerToken
where
    C: AsRequestContext,
{
    fn from_context_part(context: &mut C) -> Result<Self, McpError> {
        let token = context
            .as_request_context()
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| parts.headers.get(axum::http::header::AUTHORIZATION))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::to_string);
        Ok(BearerToken(token))
    }
}

fn ok_json(value: Value) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn err_text(message: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(message.into())]))
}

fn to_result(outcome: Result<Value, String>) -> Result<CallToolResult, McpError> {
    match outcome {
        Ok(value) => ok_json(value),
        Err(message) => err_text(message),
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListDecksRequest {
    #[schemars(description = "ID of the game (project) to list decks for, from list_games")]
    pub game_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListCardsRequest {
    #[schemars(description = "ID of the deck to list cards from, from list_decks")]
    pub deck_id: String,
    #[schemars(
        description = "Optional status filter: 'pile' (backlog, not started), 'hand' (actively being worked, max 5 per deck), or 'done' (completed). Omit to return cards of every status."
    )]
    pub status: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateCardRequest {
    #[schemars(description = "ID of the deck this card belongs to, from list_decks")]
    pub deck_id: String,
    #[schemars(description = "Short, actionable title, e.g. 'Add retry logic to upload handler'")]
    pub title: String,
    #[schemars(
        description = "Technical description with enough context that a developer could implement this without further clarification -- relevant files, functions, edge cases, or constraints. Omit only for genuinely self-explanatory cards."
    )]
    pub description: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateCardRequest {
    #[schemars(description = "ID of the card to update")]
    pub card_id: String,
    #[schemars(description = "New title. Omit to leave the existing title unchanged.")]
    pub title: Option<String>,
    #[schemars(description = "New description. Omit to leave the existing description unchanged.")]
    pub description: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddJokerRequest {
    #[schemars(
        description = "ID of the card that has the dependency -- it cannot be completed until joker_id is done"
    )]
    pub card_id: String,
    #[schemars(description = "ID of the card that must be completed first (the blocking card)")]
    pub joker_id: String,
    #[schemars(
        description = "Optional explicit 1-based sequence position among card_id's existing jokers. Omit to append after the current last joker. Must not collide with an existing order value for this card, or the call fails."
    )]
    pub order: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RemoveJokerRequest {
    #[schemars(description = "ID of the card the dependency is removed from")]
    pub card_id: String,
    #[schemars(description = "ID of the joker (blocking card) to unlink from card_id")]
    pub joker_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CardIdRequest {
    #[schemars(description = "ID of the card")]
    pub card_id: String,
}

#[derive(Clone)]
pub struct CardflowMcpServer {
    client: CardflowClient,
}

#[tool_router]
impl CardflowMcpServer {
    pub fn new(client: CardflowClient) -> Self {
        Self { client }
    }

    #[tool(
        description = "List every game (top-level project) visible to this token's user. Takes no parameters. Games have no parent -- decks and cards live underneath them. Call this first to discover game_id values before listing decks."
    )]
    async fn list_games(
        &self,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.client.list_games(&token).await)
    }

    #[tool(
        description = "List the decks (epics/sprints) within one game. Requires game_id from list_games. Each deck has its own independent hand (max 5 active cards, ranked by priority 1-5) and pile (backlog)."
    )]
    async fn list_decks(
        &self,
        Parameters(ListDecksRequest { game_id }): Parameters<ListDecksRequest>,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.client.list_decks(&token, &game_id).await)
    }

    #[tool(
        description = "List the cards (user stories) in a deck, optionally filtered by status. Requires deck_id from list_decks. Status values: 'pile' (backlog, not started), 'hand' (actively being worked), 'done' (completed, has completed_at set). Always call this before create_card to check for existing cards and avoid creating duplicates."
    )]
    async fn list_cards(
        &self,
        Parameters(ListCardsRequest { deck_id, status }): Parameters<ListCardsRequest>,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        match self.client.list_cards(&token, &deck_id).await {
            Ok(Value::Array(cards)) => {
                let filtered = match status {
                    Some(status) => cards
                        .into_iter()
                        .filter(|card| {
                            card.get("status").and_then(Value::as_str) == Some(status.as_str())
                        })
                        .collect(),
                    None => cards,
                };
                ok_json(Value::Array(filtered))
            }
            Ok(other) => ok_json(other),
            Err(message) => err_text(message),
        }
    }

    #[tool(
        description = "Create a new card (user story) in a deck's pile (backlog). It always starts with status='pile' and no priority -- it is not added to the hand automatically. Call list_cards first to avoid duplicating existing work. title should be short and actionable; description should carry enough technical detail that a developer could implement it without asking follow-up questions."
    )]
    async fn create_card(
        &self,
        Parameters(CreateCardRequest {
            deck_id,
            title,
            description,
        }): Parameters<CreateCardRequest>,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        to_result(
            self.client
                .create_card(&token, &deck_id, &title, description.as_deref())
                .await,
        )
    }

    #[tool(
        description = "Update the title and/or description of an existing card. Provide at least one of title/description; whichever is omitted is left unchanged. Does not touch status, priority, or joker dependencies -- use add_joker, remove_joker, complete_card, or return_card for those."
    )]
    async fn update_card(
        &self,
        Parameters(UpdateCardRequest {
            card_id,
            title,
            description,
        }): Parameters<UpdateCardRequest>,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        if title.is_none() && description.is_none() {
            return err_text("provide at least one of title or description to update");
        }
        to_result(
            self.client
                .update_card(&token, &card_id, title.as_deref(), description.as_deref())
                .await,
        )
    }

    #[tool(
        description = "Add a joker -- a blocking dependency -- meaning card_id cannot be completed until joker_id is completed first. Both cards must already exist and belong to the same deck: create every card in a batch first (so all card_id values exist), then wire jokers afterward. Jokers can themselves have jokers, forming a dependency chain of any depth, but a card can never depend on itself or on something that already (directly or transitively) depends on it -- the API rejects cycles with an error. Only create a joker when a genuine blocking relationship exists, not merely to express priority or grouping."
    )]
    async fn add_joker(
        &self,
        Parameters(AddJokerRequest {
            card_id,
            joker_id,
            order,
        }): Parameters<AddJokerRequest>,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        to_result(
            self.client
                .add_joker(&token, &card_id, &joker_id, order)
                .await,
        )
    }

    #[tool(
        description = "Remove a previously-added joker dependency between card_id and joker_id. This only removes the dependency edge -- it does not delete either card."
    )]
    async fn remove_joker(
        &self,
        Parameters(RemoveJokerRequest { card_id, joker_id }): Parameters<RemoveJokerRequest>,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.client.remove_joker(&token, &card_id, &joker_id).await)
    }

    #[tool(
        description = "Mark a card as done. Only works on a card currently in the hand (status='hand') whose own direct joker dependencies are all already done -- if any joker is unresolved, the call fails with an error. Completing a card clears its hand slot/priority and sets completed_at."
    )]
    async fn complete_card(
        &self,
        Parameters(CardIdRequest { card_id }): Parameters<CardIdRequest>,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.client.complete_card(&token, &card_id).await)
    }

    #[tool(
        description = "Return a card from the hand back to the pile, clearing its priority. Only works on a hand card that is the root of its stack (drawn directly, not pulled in only as someone else's joker)."
    )]
    async fn return_card(
        &self,
        Parameters(CardIdRequest { card_id }): Parameters<CardIdRequest>,
        BearerToken(token): BearerToken,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.client.return_card(&token, &card_id).await)
    }
}

#[tool_handler]
impl ServerHandler for CardflowMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(INSTRUCTIONS.to_string())
    }
}
