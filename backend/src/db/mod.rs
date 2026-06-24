use std::str::FromStr;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

pub type DbPool = SqlitePool;

pub async fn init_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!("./src/db/migrations").run(&pool).await?;
    Ok(pool)
}
