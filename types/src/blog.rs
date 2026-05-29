use crate::{Block, ImageVariant};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Editorial state of a blog post.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Draft,
    Published,
}

/// Payload to create a blog post from the CRM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPost {
    pub slug: String,
    pub title: String,
    /// Structured block document (source of record).
    #[serde(default)]
    pub blocks: Vec<Block>,
    #[serde(default = "draft")]
    pub status: PostStatus,
}

fn draft() -> PostStatus {
    PostStatus::Draft
}

/// Partial update for a post (any omitted field is left unchanged).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdatePost {
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub blocks: Option<Vec<Block>>,
    #[serde(default)]
    pub status: Option<PostStatus>,
}

/// Result of uploading an inline post image (`POST /api/admin/blog-assets`).
/// Runs the same ingest as gallery photos (responsive AVIF+JPEG renditions +
/// a ThumbHash blur-up), so blog images are optimized identically — but no
/// gallery/DB row is created. The CRM copies these fields onto the image block,
/// which stays the sole record of the asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadedImage {
    /// Largest JPEG rendition: the broadly-compatible `<img>` fallback and the
    /// editor's inline preview src.
    pub src: String,
    /// Responsive `(format, width)` renditions, same shape as gallery photos.
    #[serde(default)]
    pub variants: Vec<ImageVariant>,
    /// Precomputed blur-up placeholder (`data:image/png;base64,…`), painted
    /// before the full image loads.
    #[serde(default)]
    pub placeholder: String,
    pub width: u32,
    pub height: u32,
}

/// A blog post authored in the CRM and rendered statically by the public site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlogPost {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    /// Structured block document — the source of record the CRM edits.
    pub blocks: Vec<Block>,
    /// Server-rendered HTML (from `blocks` via `syle-render`). The public site
    /// injects this so it renders identically to the CRM preview; it is derived
    /// output and ignored on write.
    #[serde(default)]
    pub body_html: String,
    pub status: PostStatus,
    /// Unix seconds; `None` until first published.
    pub published_at: Option<i64>,
}
