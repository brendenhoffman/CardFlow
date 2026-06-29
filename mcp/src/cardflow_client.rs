use serde_json::{json, Value};

/// Thin HTTP client for the Cardflow backend API. Every method takes the
/// bearer token presented by the *connecting MCP client* for this call (an
/// API token or an OAuth access token) and forwards it as-is to Cardflow,
/// which does all real validation -- this client never inspects or verifies
/// it. If the connecting client didn't present one, `fallback_token` (the
/// static `CARDFLOW_TOKEN`, if configured) is used instead, preserving the
/// original single-tenant deployment model where the MCP server itself holds
/// one fixed identity.
///
/// Every method returns `Err(String)` with a human-readable message on
/// failure so MCP tools can surface it directly to the calling model without
/// extra mapping.
#[derive(Clone)]
pub struct CardflowClient {
    http: reqwest::Client,
    base_url: String,
    fallback_token: Option<String>,
}

impl CardflowClient {
    pub fn new(base_url: String, fallback_token: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            fallback_token,
        }
    }

    fn resolve_token<'a>(&'a self, per_call: &'a Option<String>) -> Result<&'a str, String> {
        per_call
            .as_deref()
            .or(self.fallback_token.as_deref())
            .ok_or_else(|| {
                "no Cardflow credentials available: connect with an Authorization header \
                 (API token or OAuth access token) or configure CARDFLOW_TOKEN on the server"
                    .to_string()
            })
    }

    async fn request(
        &self,
        token: &Option<String>,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, String> {
        let token = self.resolve_token(token)?;
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.request(method, &url).bearer_auth(token);
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

    pub async fn list_games(&self, token: &Option<String>) -> Result<Value, String> {
        self.request(token, reqwest::Method::GET, "/games", None)
            .await
    }

    pub async fn list_decks(&self, token: &Option<String>, game_id: &str) -> Result<Value, String> {
        self.request(
            token,
            reqwest::Method::GET,
            &format!("/games/{game_id}/decks"),
            None,
        )
        .await
    }

    pub async fn list_cards(&self, token: &Option<String>, deck_id: &str) -> Result<Value, String> {
        self.request(
            token,
            reqwest::Method::GET,
            &format!("/decks/{deck_id}/cards"),
            None,
        )
        .await
    }

    pub async fn create_card(
        &self,
        token: &Option<String>,
        deck_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<Value, String> {
        self.request(
            token,
            reqwest::Method::POST,
            &format!("/decks/{deck_id}/cards"),
            Some(json!({ "title": title, "description": description })),
        )
        .await
    }

    pub async fn update_card(
        &self,
        token: &Option<String>,
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
            token,
            reqwest::Method::PATCH,
            &format!("/cards/{card_id}"),
            Some(body),
        )
        .await
    }

    pub async fn add_joker(
        &self,
        token: &Option<String>,
        card_id: &str,
        joker_id: &str,
        order: Option<i64>,
    ) -> Result<Value, String> {
        let mut body = json!({ "joker_id": joker_id });
        if let Some(order) = order {
            body["order"] = json!(order);
        }
        self.request(
            token,
            reqwest::Method::POST,
            &format!("/cards/{card_id}/jokers"),
            Some(body),
        )
        .await
    }

    pub async fn remove_joker(
        &self,
        token: &Option<String>,
        card_id: &str,
        joker_id: &str,
    ) -> Result<Value, String> {
        self.request(
            token,
            reqwest::Method::DELETE,
            &format!("/cards/{card_id}/jokers/{joker_id}"),
            None,
        )
        .await
    }

    pub async fn complete_card(
        &self,
        token: &Option<String>,
        card_id: &str,
    ) -> Result<Value, String> {
        self.request(
            token,
            reqwest::Method::POST,
            &format!("/cards/{card_id}/complete"),
            None,
        )
        .await
    }

    pub async fn return_card(
        &self,
        token: &Option<String>,
        card_id: &str,
    ) -> Result<Value, String> {
        self.request(
            token,
            reqwest::Method::POST,
            &format!("/cards/{card_id}/return"),
            None,
        )
        .await
    }
}
