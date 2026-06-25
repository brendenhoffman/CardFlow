mod auth;
mod db;
mod errors;
mod middleware;
mod models;
mod routes;

use axum::Router;
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://cardflow.db".to_string());
    let pool = db::init_pool(&database_url).await?;

    bootstrap_admin(&pool).await?;

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
        .merge(routes::auth::public_router())
        .merge(protected)
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

/// Creates the first admin user from ADMIN_USERNAME/ADMIN_PASSWORD if no admin exists yet.
async fn bootstrap_admin(pool: &SqlitePool) -> anyhow::Result<()> {
    let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(pool)
        .await?;
    if admin_count > 0 {
        return Ok(());
    }

    let (username, password) = match (
        std::env::var("ADMIN_USERNAME").ok(),
        std::env::var("ADMIN_PASSWORD").ok(),
    ) {
        (Some(username), Some(password)) => (username, password),
        _ => {
            tracing::warn!(
                "no admin user exists and ADMIN_USERNAME/ADMIN_PASSWORD are not set; skipping bootstrap"
            );
            return Ok(());
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = auth::hash_password(&password)?;
    let created_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, totp_secret, role, created_at) VALUES (?, ?, ?, NULL, 'admin', ?)",
    )
    .bind(&id)
    .bind(&username)
    .bind(&password_hash)
    .bind(&created_at)
    .execute(pool)
    .await?;

    tracing::info!(username = %username, "created first admin user");
    Ok(())
}
