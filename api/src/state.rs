use crate::site::GithubConfig;
use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use webauthn_rs::prelude::Webauthn;

// One encode already uses four rav1e threads. Queue additional uploads instead
// of letting two large images monopolize the six-core VPS and starve the API.
const MAX_CONCURRENT_IMAGE_JOBS: usize = 1;

/// Shared application state. Cheap to clone (pool, `webauthn` and the reqwest
/// client inside `github` are all `Arc`-backed).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// Root directory for ingested originals + derivatives.
    pub media_dir: PathBuf,
    /// WebAuthn relying party, shared across all passkey ceremonies.
    pub webauthn: Arc<Webauthn>,
    /// Public-site rebuild trigger; `None` when no token is configured, which
    /// leaves the feature dormant rather than failing the API.
    pub github: Option<GithubConfig>,
    /// Bounds simultaneous decode/resize/encode jobs so a few authenticated
    /// large uploads cannot exhaust the process's CPU and memory.
    pub image_jobs: Arc<Semaphore>,
}

impl AppState {
    pub fn new(pool: PgPool, media_dir: PathBuf, webauthn: Webauthn) -> Self {
        Self {
            pool,
            media_dir,
            webauthn: Arc::new(webauthn),
            github: GithubConfig::from_env(),
            image_jobs: Arc::new(Semaphore::new(MAX_CONCURRENT_IMAGE_JOBS)),
        }
    }
}
