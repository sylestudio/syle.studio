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
