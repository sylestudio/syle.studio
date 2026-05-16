use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Editorial state of a blog post.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Draft,
    Published,
}

/// A blog post authored in the CRM and rendered statically by the public site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlogPost {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    /// Markdown/MDX source of record.
    pub body_md: String,
    pub status: PostStatus,
    /// Unix seconds; `None` until first published.
    pub published_at: Option<i64>,
}
