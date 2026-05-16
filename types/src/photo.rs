use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Encoded output format of a derivative produced by the ingest pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Avif,
    Webp,
}

/// One responsive derivative: a `(format, width)` rendition at a stable path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageVariant {
    pub format: ImageFormat,
    pub width: u32,
    /// Content-hashed, immutable path served behind the CDN.
    pub path: String,
}

/// A photo plus its precomputed placeholder and responsive derivatives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Photo {
    pub id: Uuid,
    pub gallery_id: Uuid,
    pub alt: String,
    /// ThumbHash string painted instantly before the image loads.
    pub thumbhash: String,
    pub width: u32,
    pub height: u32,
    /// Display order within the gallery; lower sorts first.
    pub position: i32,
    pub variants: Vec<ImageVariant>,
}
