mod cardflow_client;
mod oauth;
mod tools;

use std::net::SocketAddr;

use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tokio_util::sync::CancellationToken;

use cardflow_client::CardflowClient;
use oauth::OAuthState;
use tools::CardflowMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cardflow_url = std::env::var("CARDFLOW_URL")
        .unwrap_or_else(|_| "http://cardflow-backend:3001".to_string());
    let cardflow_token = std::env::var("CARDFLOW_TOKEN").ok();
    let mcp_port: u16 = std::env::var("MCP_PORT")
        .unwrap_or_else(|_| "3778".to_string())
        .parse()
        .map_err(|_| anyhow::anyhow!("MCP_PORT must be a valid port number"))?;

    let client = CardflowClient::new(cardflow_url.clone(), cardflow_token);
    let oauth_state = build_oauth_state(&cardflow_url)?;

    let bind: SocketAddr = ([0, 0, 0, 0], mcp_port).into();
    let config = SseServerConfig {
        bind,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (sse_server, sse_router) = SseServer::new(config);
    let router = match oauth_state {
        Some(state) => {
            tracing::info!(
                "OAuth 2.1 support enabled (issuer: {})",
                state.mcp_public_url
            );
            sse_router.merge(oauth::router(state))
        }
        None => {
            tracing::info!(
                "OAuth not configured (set OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, MCP_PUBLIC_URL, \
                 and CARDFLOW_PUBLIC_URL to enable it) -- API token bearer auth only"
            );
            sse_router
        }
    };

    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
    let ct = sse_server.config.ct.child_token();

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        ct.cancelled().await;
        tracing::info!("mcp sse server cancelled");
    });

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "mcp sse server shut down with error");
        }
    });

    tracing::info!("cardflow-mcp listening on {bind}, talking to Cardflow at {cardflow_url}");
    let shutdown_token = sse_server.with_service(move || CardflowMcpServer::new(client.clone()));

    tokio::signal::ctrl_c().await?;
    shutdown_token.cancel();
    Ok(())
}

/// OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, MCP_PUBLIC_URL, and CARDFLOW_PUBLIC_URL
/// are required together to enable OAuth, or left entirely unset to disable
/// it (API token bearer auth keeps working either way). Note that
/// OAUTH_CLIENT_ID/SECRET aren't actually used by this process for crypto --
/// /authorize and /token are pure proxies to the backend, which holds its own
/// copy of the same two values and does the real validation. They're read
/// here only as a presence signal for this all-or-nothing startup gate.
fn build_oauth_state(cardflow_url: &str) -> anyhow::Result<Option<OAuthState>> {
    let client_id = non_empty_env("OAUTH_CLIENT_ID");
    let client_secret = non_empty_env("OAUTH_CLIENT_SECRET");
    let mcp_public_url = non_empty_env("MCP_PUBLIC_URL");
    let cardflow_public_url = non_empty_env("CARDFLOW_PUBLIC_URL");

    let present = [
        client_id.is_some(),
        client_secret.is_some(),
        mcp_public_url.is_some(),
        cardflow_public_url.is_some(),
    ];

    if !present.iter().any(|p| *p) {
        return Ok(None);
    }
    if !present.iter().all(|p| *p) {
        anyhow::bail!(
            "OAuth is partially configured: OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, MCP_PUBLIC_URL, \
             and CARDFLOW_PUBLIC_URL must all be set together to enable it, or all left unset to disable it"
        );
    }

    Ok(Some(OAuthState {
        http: reqwest::Client::new(),
        cardflow_url: cardflow_url.to_string(),
        cardflow_public_url: cardflow_public_url.expect("checked above"),
        mcp_public_url: mcp_public_url.expect("checked above"),
    }))
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}
