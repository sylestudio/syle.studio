//! Shared DTOs that form the contract between the Axum API and the Leptos CRM.
//!
//! This crate is `serde`-only and target-agnostic (compiles for host and
//! `wasm32`); it carries no I/O, no DB, no business logic.

mod auth;
mod block;
mod blog;
pub mod endpoints;
mod gallery;
mod history;
mod photo;
mod slug;

pub use auth::{LoginRequest, SessionToken, User};
pub use block::{Block, Mark, Span};
pub use history::History;
pub use blog::{BlogPost, NewPost, PostStatus, UpdatePost, UploadedImage};
pub use gallery::{Gallery, GalleryDetail, NewGallery, UpdateGallery};
pub use photo::{ImageFormat, ImageVariant, Photo, Reorder, UpdatePhoto};
pub use slug::is_valid_slug;

use serde::{Deserialize, Serialize};

/// ThumbHash placeholder string, computed server-side at ingest.
pub type ThumbHash = String;

/// Liveness payload for `GET /health`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub ok: bool,
}
