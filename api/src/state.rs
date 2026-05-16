use sqlx::PgPool;
use std::path::PathBuf;

/// Shared application state. Cheap to clone (pool is `Arc` internally).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// Root directory for ingested originals + derivatives.
    pub media_dir: PathBuf,
}

impl AppState {
    pub fn new(pool: PgPool, media_dir: PathBuf) -> Self {
        Self { pool, media_dir }
    }
}
