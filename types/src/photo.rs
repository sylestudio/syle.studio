use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Partial update for a photo (any omitted field is left unchanged).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdatePhoto {
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default)]
    pub position: Option<i32>,
}

/// New ordering: `position` becomes each id's index in this list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reorder {
    pub ids: Vec<Uuid>,
}

/// Encoded output format of a derivative produced by the ingest pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Avif,
    Jpeg,
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
