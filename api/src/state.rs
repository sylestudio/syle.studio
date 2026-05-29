use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;
use webauthn_rs::prelude::Webauthn;

/// Shared application state. Cheap to clone (pool and `webauthn` are `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// Root directory for ingested originals + derivatives.
    pub media_dir: PathBuf,
    /// WebAuthn relying party, shared across all passkey ceremonies.
    pub webauthn: Arc<Webauthn>,
}

impl AppState {
    pub fn new(pool: PgPool, media_dir: PathBuf, webauthn: Webauthn) -> Self {
        Self {
            pool,
            media_dir,
            webauthn: Arc::new(webauthn),
        }
    }
}
