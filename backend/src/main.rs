mod auth;
mod db;
mod errors;
mod middleware;
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
    auth::init_jwt_secret(&database_url)?;
    let pool = db::init_pool(&database_url).await?;

    let protected = Router::new()
        .merge(routes::auth::protected_router())
        .merge(routes::users::router())
        .merge(routes::games::router())
        .merge(routes::decks::router())
        .merge(routes::cards::router())
        .layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            middleware::require_auth,
        ));

    let app = Router::new()
        .merge(routes::setup::router())
        .merge(routes::auth::public_router())
        .merge(protected)
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
