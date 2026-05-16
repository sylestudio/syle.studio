//! Postgres connection pool and migration runner.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Open a bounded connection pool to `database_url`.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}

/// Apply all pending migrations embedded from `core/migrations`.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
