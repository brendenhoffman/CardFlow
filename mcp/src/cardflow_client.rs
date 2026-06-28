use serde_json::{json, Value};

/// Thin HTTP client for the Cardflow backend API, authenticated with a
/// long-lived API token (see `/api-tokens` on the backend). Every method
/// returns `Err(String)` with a human-readable message on failure so MCP
/// tools can surface it directly to the calling model without extra mapping.
#[derive(Clone)]
pub struct CardflowClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl CardflowClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            token,
        }
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.request(method, &url).bearer_auth(&self.token);
        if let Some(body) = body {
            req = req.json(&body);
        }

        let res = req
            .send()
            .await
            .map_err(|e| format!("request to Cardflow backend failed: {e}"))?;
        let status = res.status();
        let text = res
            .text()
            .await
            .map_err(|e| format!("failed to read Cardflow response body: {e}"))?;

        if !status.is_success() {
            let message = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_else(|| text.clone());
            return Err(format!("Cardflow API returned {status}: {message}"));
        }

        if text.trim().is_empty() {
            Ok(Value::Null)
        } else {
            serde_json::from_str(&text)
                .map_err(|e| format!("failed to parse Cardflow response as JSON: {e}"))
        }
    }

    pub async fn list_games(&self) -> Result<Value, String> {
        self.request(reqwest::Method::GET, "/games", None).await
    }

    pub async fn list_decks(&self, game_id: &str) -> Result<Value, String> {
        self.request(
            reqwest::Method::GET,
            &format!("/games/{game_id}/decks"),
            None,
        )
        .await
    }

    pub async fn list_cards(&self, deck_id: &str) -> Result<Value, String> {
        self.request(
            reqwest::Method::GET,
            &format!("/decks/{deck_id}/cards"),
            None,
        )
        .await
    }

    pub async fn create_card(
        &self,
        deck_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<Value, String> {
        self.request(
            reqwest::Method::POST,
            &format!("/decks/{deck_id}/cards"),
            Some(json!({ "title": title, "description": description })),
        )
        .await
    }

    pub async fn update_card(
        &self,
        card_id: &str,
        title: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value, String> {
        let mut body = json!({});
        if let Some(title) = title {
            body["title"] = json!(title);
        }
        if let Some(description) = description {
            body["description"] = json!(description);
        }
        self.request(
            reqwest::Method::PATCH,
            &format!("/cards/{card_id}"),
            Some(body),
        )
        .await
    }

    pub async fn add_joker(
        &self,
        card_id: &str,
        joker_id: &str,
        order: Option<i64>,
    ) -> Result<Value, String> {
        let mut body = json!({ "joker_id": joker_id });
        if let Some(order) = order {
            body["order"] = json!(order);
        }
        self.request(
            reqwest::Method::POST,
            &format!("/cards/{card_id}/jokers"),
            Some(body),
        )
        .await
    }

    pub async fn remove_joker(&self, card_id: &str, joker_id: &str) -> Result<Value, String> {
        self.request(
            reqwest::Method::DELETE,
            &format!("/cards/{card_id}/jokers/{joker_id}"),
            None,
        )
        .await
    }

    pub async fn complete_card(&self, card_id: &str) -> Result<Value, String> {
        self.request(
            reqwest::Method::POST,
            &format!("/cards/{card_id}/complete"),
            None,
        )
        .await
    }

    pub async fn return_card(&self, card_id: &str) -> Result<Value, String> {
        self.request(
            reqwest::Method::POST,
            &format!("/cards/{card_id}/return"),
            None,
        )
        .await
    }
}
