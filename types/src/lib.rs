//! Shared DTOs that form the contract between the Axum API and the Leptos CRM.
//!
//! This crate is `serde`-only and target-agnostic (compiles for host and
//! `wasm32`); it carries no I/O, no DB, no business logic.

mod auth;
mod blog;
mod gallery;
mod photo;

pub use auth::{LoginRequest, SessionToken, User};
pub use blog::{BlogPost, PostStatus};
pub use gallery::Gallery;
pub use photo::{ImageFormat, ImageVariant, Photo};

use serde::{Deserialize, Serialize};

/// ThumbHash placeholder string, computed server-side at ingest.
pub type ThumbHash = String;

/// Liveness payload for `GET /health`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub ok: bool,
}
