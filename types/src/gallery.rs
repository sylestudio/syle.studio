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
    /// Hero lede: intro prose shown on the public project page.
    pub description: String,
    /// Closing "Notas del proyecto" prose on the public project page.
    pub notes: String,
    /// Discipline label (e.g. "Dirección · Prenda"); empty when unset.
    pub category: String,
    /// Project year; `None` until set in the CRM.
    pub year: Option<i32>,
}

/// A gallery together with its ordered photos (public detail response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GalleryDetail {
    pub gallery: Gallery,
    pub photos: Vec<Photo>,
}

/// Partial update for a gallery (any omitted field is left unchanged).
///
/// `year` follows the same `None` = unchanged rule as every other field, so a
/// once-set year cannot be cleared back to NULL via PATCH — a deliberate
/// trade for keeping the simple COALESCE merge the partial-update suite locks in.
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
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
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
