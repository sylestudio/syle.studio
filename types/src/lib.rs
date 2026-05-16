//! Shared DTOs that form the contract between the Axum API and the Leptos CRM.

use serde::{Deserialize, Serialize};

/// BlurHash-style placeholder string, computed server-side at ingest.
pub type ThumbHash = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub ok: bool,
}
