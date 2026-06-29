mod cardflow_client;
mod oauth;
mod tools;

use std::net::SocketAddr;

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
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

    let ct = CancellationToken::new();
    let mcp_service = StreamableHttpService::new(
        move || Ok(CardflowMcpServer::new(client.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_cancellation_token(ct.child_token())
            .with_allowed_hosts(allowed_hosts()),
    );

    let mut router = axum::Router::new().nest_service("/mcp", mcp_service);
    router = match oauth_state {
        Some(state) => {
            tracing::info!(
                "OAuth 2.1 support enabled (issuer: {})",
                state.mcp_public_url
            );
            router.merge(oauth::router(state))
        }
        None => {
            tracing::info!(
                "OAuth not configured (set OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, MCP_PUBLIC_URL, \
                 and CARDFLOW_PUBLIC_URL to enable it) -- API token bearer auth only"
            );
            router
        }
    };

    let bind: SocketAddr = ([0, 0, 0, 0], mcp_port).into();
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!("cardflow-mcp listening on {bind}/mcp, talking to Cardflow at {cardflow_url}");

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("mcp server shutting down");
            ct.cancel();
        })
        .await?;

    Ok(())
}

/// OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, MCP_PUBLIC_URL, and CARDFLOW_PUBLIC_URL
/// are required together to enable OAuth, or left entirely unset to disable
/// it (API token bearer auth keeps working either way). Note that
/// OAUTH_CLIENT_ID/SECRET aren't actually used by this process for crypto --
/// /authorize and /token are pure proxies to the backend, which holds its own
/// copy of the same two values and does the real validation. They're read
/// here only as a presence signal for this all-or-nothing startup gate.
///
/// MCP_PUBLIC_URL doubles as the source for the allowed Host header (see
/// `allowed_hosts`), which is useful even without OAuth -- so "OAuth wanted
/// at all" is judged by the other three vars, not this one. Setting only
/// MCP_PUBLIC_URL (for the Host header fix, on a deployment that otherwise
/// just uses CARDFLOW_TOKEN) is intentionally not an error.
fn build_oauth_state(cardflow_url: &str) -> anyhow::Result<Option<OAuthState>> {
    let client_id = non_empty_env("OAUTH_CLIENT_ID");
    let client_secret = non_empty_env("OAUTH_CLIENT_SECRET");
    let mcp_public_url = non_empty_env("MCP_PUBLIC_URL");
    let cardflow_public_url = non_empty_env("CARDFLOW_PUBLIC_URL");

    let oauth_specific_present = [
        client_id.is_some(),
        client_secret.is_some(),
        cardflow_public_url.is_some(),
    ];

    if !oauth_specific_present.iter().any(|p| *p) {
        return Ok(None);
    }
    if !oauth_specific_present.iter().all(|p| *p) || mcp_public_url.is_none() {
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

/// rmcp's Streamable HTTP transport rejects requests whose `Host` header isn't
/// on an allowlist (DNS-rebinding protection), defaulting to loopback only --
/// which rejects every request once this server is reachable under its real
/// public hostname. Extend that default with the host parsed from
/// MCP_PUBLIC_URL, if set, rather than adding a separate env var for it. An
/// allowed entry with no port matches that host on any port, so only the
/// bare hostname is needed.
fn allowed_hosts() -> Vec<String> {
    let mut hosts = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];

    if let Some(mcp_public_url) = non_empty_env("MCP_PUBLIC_URL") {
        match url::Url::parse(&mcp_public_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
        {
            Some(host) => hosts.push(host),
            None => tracing::warn!(
                "MCP_PUBLIC_URL ({mcp_public_url}) could not be parsed for its hostname; \
                 only loopback Host headers will be accepted"
            ),
        }
    }

    hosts
}
