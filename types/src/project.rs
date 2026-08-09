use crate::UploadedImage;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// A standalone project card in the public portfolio grid.
///
/// Unlike a gallery, it owns no detail page or photo collection: selecting the
/// card navigates straight to `url` (for example `/proyectos/dango`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub url: String,
    /// Discipline label shown above the title; empty when unset.
    pub category: String,
    /// Shared display-order space with galleries; lower sorts first.
    pub position: i32,
    pub published: bool,
    /// Responsive cover uploaded through the asset pipeline, without a gallery.
    pub cover: Option<UploadedImage>,
}

/// Payload to create a standalone project from the CRM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProject {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub published: bool,
    #[serde(default)]
    pub cover: Option<UploadedImage>,
}

/// Partial update for a standalone project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateProject {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub published: Option<bool>,
    #[serde(default)]
    pub cover: Option<UploadedImage>,
    /// `true` explicitly removes the current cover; omission leaves it intact.
    #[serde(default)]
    pub clear_cover: bool,
}

/// Accept a site-relative path or an absolute HTTP(S) URL.
///
/// Site-relative paths must begin with one slash. Protocol-relative URLs and
/// backslashes are rejected so a CRM value can safely become an anchor href.
pub fn is_valid_project_url(value: &str) -> bool {
    let value = value.trim();
    if value.contains('\\') || value.chars().any(char::is_control) {
        return false;
    }
    if value.starts_with('/') {
        if value.starts_with("//") {
            return false;
        }
        let Ok(base) = Url::parse("https://syle.studio/") else {
            return false;
        };
        return base.join(value).is_ok();
    }

    let Ok(url) = Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
}

#[cfg(test)]
mod tests {
    use super::is_valid_project_url;

    #[test]
    fn project_urls_allow_internal_and_http_destinations() {
        assert!(is_valid_project_url("/proyectos/dango"));
        assert!(is_valid_project_url("/"));
        assert!(is_valid_project_url("https://example.com/work?id=1"));
        assert!(is_valid_project_url("http://localhost:4321/demo"));
    }

    #[test]
    fn project_urls_reject_unsafe_or_ambiguous_destinations() {
        assert!(!is_valid_project_url(""));
        assert!(!is_valid_project_url("proyectos/dango"));
        assert!(!is_valid_project_url("//example.com/work"));
        assert!(!is_valid_project_url("/proyectos\\dango"));
        assert!(!is_valid_project_url("javascript:alert(1)"));
        assert!(!is_valid_project_url("https://user:pass@example.com/work"));
    }
}
