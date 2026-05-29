//! Structured block document model for blog posts.
//!
//! A post body is a `Vec<Block>`. Each text-bearing block carries inline
//! content as `Vec<Span>` (a run of text plus its marks), mirroring the
//! ProseMirror/BlockNote "inline content" shape. This is the source of record
//! the CRM edits and the renderer (`syle-render`) turns into HTML — so the
//! public site and the CRM preview render from the exact same data.

use serde::{Deserialize, Serialize};

/// Inline formatting applied to a run of text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Mark {
    Bold,
    Italic,
    Code,
    Strike,
    Link { href: String },
}

/// A run of text with zero or more inline marks ("inline content").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<Mark>,
}

impl Span {
    /// A span of unmarked text.
    pub fn plain(text: impl Into<String>) -> Self {
        Span {
            text: text.into(),
            marks: Vec::new(),
        }
    }
}

/// One block in a post document. Tagged by `type` on the wire.
///
/// Consecutive `BulletItem`/`NumberedItem`/`Todo` blocks are grouped into a
/// single list by the renderer; in the model each item is its own block, which
/// keeps editing uniform (every block is one editable unit, Notion-style).
/// `Heading.level` is clamped to 1..=3 by the renderer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Paragraph {
        id: String,
        #[serde(default)]
        content: Vec<Span>,
    },
    Heading {
        id: String,
        level: u8,
        #[serde(default)]
        content: Vec<Span>,
    },
    Quote {
        id: String,
        #[serde(default)]
        content: Vec<Span>,
    },
    BulletItem {
        id: String,
        #[serde(default)]
        content: Vec<Span>,
    },
    NumberedItem {
        id: String,
        #[serde(default)]
        content: Vec<Span>,
    },
    Todo {
        id: String,
        #[serde(default)]
        checked: bool,
        #[serde(default)]
        content: Vec<Span>,
    },
    Code {
        id: String,
        #[serde(default)]
        language: String,
        #[serde(default)]
        code: String,
    },
    Divider {
        id: String,
    },
    Image {
        id: String,
        src: String,
        #[serde(default)]
        alt: String,
        #[serde(default)]
        caption: Vec<Span>,
    },
    Callout {
        id: String,
        #[serde(default)]
        emoji: String,
        #[serde(default)]
        content: Vec<Span>,
    },
}

impl Block {
    /// Stable identity, used as the keyed-list key in the editor.
    pub fn id(&self) -> &str {
        match self {
            Block::Paragraph { id, .. }
            | Block::Heading { id, .. }
            | Block::Quote { id, .. }
            | Block::BulletItem { id, .. }
            | Block::NumberedItem { id, .. }
            | Block::Todo { id, .. }
            | Block::Code { id, .. }
            | Block::Divider { id }
            | Block::Image { id, .. }
            | Block::Callout { id, .. } => id,
        }
    }
}
