use crate::Photo;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A named, ordered collection of photos (single-catalog; no tenant scoping).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gallery {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    /// Display order; lower sorts first.
    pub position: i32,
    pub published: bool,
}

/// A gallery together with its ordered photos (public detail response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GalleryDetail {
    pub gallery: Gallery,
    pub photos: Vec<Photo>,
}

/// Partial update for a gallery (any omitted field is left unchanged).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateGallery {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub published: Option<bool>,
    #[serde(default)]
    pub position: Option<i32>,
}

/// Payload to create a gallery from the CRM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGallery {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub published: bool,
}
