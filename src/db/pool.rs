use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;

use crate::config::DatabaseConfig;
use crate::error::AppResult;

pub type DbPool = Arc<PgPool>;

pub async fn create_pool(config: &DatabaseConfig) -> AppResult<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))
        .connect(&config.url)
        .await?;

    tracing::info!("Database connection pool created successfully");

    Ok(Arc::new(pool))
}

pub async fn run_migrations(pool: &PgPool) -> AppResult<()> {
    tracing::info!("Running database migrations...");

    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| crate::error::AppError::Database(sqlx::Error::Migrate(Box::new(e))))?;

    tracing::info!("Database migrations completed successfully");

    Ok(())
}
