mod cardflow_client;
mod tools;

use std::net::SocketAddr;

use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tokio_util::sync::CancellationToken;

use cardflow_client::CardflowClient;
use tools::CardflowMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cardflow_url = std::env::var("CARDFLOW_URL")
        .unwrap_or_else(|_| "http://cardflow-backend:3001".to_string());
    let cardflow_token = std::env::var("CARDFLOW_TOKEN")
        .map_err(|_| anyhow::anyhow!("CARDFLOW_TOKEN environment variable is required"))?;
    let mcp_port: u16 = std::env::var("MCP_PORT")
        .unwrap_or_else(|_| "3778".to_string())
        .parse()
        .map_err(|_| anyhow::anyhow!("MCP_PORT must be a valid port number"))?;

    let client = CardflowClient::new(cardflow_url.clone(), cardflow_token);

    let bind: SocketAddr = ([0, 0, 0, 0], mcp_port).into();
    let config = SseServerConfig {
        bind,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (sse_server, router) = SseServer::new(config);
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
