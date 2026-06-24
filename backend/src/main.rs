mod db;
mod errors;
mod models;
mod routes;

use axum::Router;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://cardflow.db".to_string());
    let pool = db::init_pool(&database_url).await?;

    let app = Router::new()
        .merge(routes::games::router())
        .merge(routes::decks::router())
        .merge(routes::cards::router())
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
