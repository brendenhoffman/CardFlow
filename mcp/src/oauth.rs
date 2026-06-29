use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use url::Url;

/// Static, single-tenant OAuth client config (no dynamic client registration
/// yet -- see README). All three fields are required together; the caller
/// only constructs this when all are present (see `main.rs`).
#[derive(Clone)]
pub struct OAuthState {
    pub http: reqwest::Client,
    /// Internal Cardflow backend URL, for the server-to-server /token proxy.
    pub cardflow_url: String,
    /// Public Cardflow (frontend) URL, for the browser-facing /authorize redirect.
    pub cardflow_public_url: String,
    /// This MCP server's own public URL, advertised in the metadata document.
    pub mcp_public_url: String,
}

pub fn router(state: OAuthState) -> Router {
    Router::new()
        .route("/.well-known/oauth-authorization-server", get(metadata))
        .route("/authorize", get(authorize_redirect))
        .route("/token", post(token_proxy))
        .with_state(state)
}

async fn metadata(State(state): State<OAuthState>) -> Json<serde_json::Value> {
    let issuer = state.mcp_public_url.trim_end_matches('/');
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["client_secret_post"],
    }))
}

/// Pure pass-through: every real validation (client_id, redirect_uri,
/// code_challenge, login) happens on the Cardflow backend. This just gets the
/// browser there, via the frontend's public URL (which proxies /api/* to the
/// backend) since CARDFLOW_URL is normally only reachable on the internal
/// Docker network, not from a user's browser.
async fn authorize_redirect(
    State(state): State<OAuthState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Redirect, (StatusCode, String)> {
    let mut url = Url::parse(&format!(
        "{}/api/oauth/authorize",
        state.cardflow_public_url.trim_end_matches('/')
    ))
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CARDFLOW_PUBLIC_URL is not a valid URL".to_string(),
        )
    })?;
    {
        let mut qp = url.query_pairs_mut();
        for (key, value) in &params {
            qp.append_pair(key, value);
        }
    }
    Ok(Redirect::to(url.as_str()))
}

/// Transparent proxy to the backend's /oauth/token, over the internal Docker
/// network. Client authentication (client_id/client_secret) and all grant
/// handling happens entirely on the backend; this never inspects the body.
async fn token_proxy(
    State(state): State<OAuthState>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let mut req = state
        .http
        .post(format!("{}/oauth/token", state.cardflow_url))
        .body(body);
    if let Some(content_type) = headers.get(CONTENT_TYPE) {
        req = req.header(CONTENT_TYPE, content_type);
    }

    match req.send().await {
        Ok(res) => {
            let status = res.status();
            match res.bytes().await {
                Ok(body) => (status, body).into_response(),
                Err(e) => (
                    StatusCode::BAD_GATEWAY,
                    format!("failed to read Cardflow response: {e}"),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("failed to reach Cardflow backend: {e}"),
        )
            .into_response(),
    }
}
