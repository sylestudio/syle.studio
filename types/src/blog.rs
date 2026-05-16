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
    pub body_md: String,
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
    pub title: Option<String>,
    #[serde(default)]
    pub body_md: Option<String>,
    #[serde(default)]
    pub status: Option<PostStatus>,
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
