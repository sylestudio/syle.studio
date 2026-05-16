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
