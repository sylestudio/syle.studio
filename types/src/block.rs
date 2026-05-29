//! Structured block document model for blog posts.
//!
//! A post body is a `Vec<Block>`. Each text-bearing block carries inline
//! content as `Vec<Span>` (a run of text plus its marks), mirroring the
//! ProseMirror/BlockNote "inline content" shape. This is the source of record
//! the CRM edits and the renderer (`syle-render`) turns into HTML — so the
//! public site and the CRM preview render from the exact same data.

use crate::ImageVariant;
use serde::{Deserialize, Serialize};

/// `skip_serializing_if` predicate: keep zero `indent` off the wire.
fn is_zero(n: &u8) -> bool {
    *n == 0
}

/// `skip_serializing_if` predicate for unset image dimensions (e.g. a pasted
/// external URL whose size we don't know).
fn is_zero_u32(n: &u32) -> bool {
    *n == 0
}

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
        /// Nesting depth (0 = top level); the renderer turns runs of items into
        /// nested `<ul>`/`<ol>` by this. Omitted from the wire when 0.
        #[serde(default, skip_serializing_if = "is_zero")]
        indent: u8,
        #[serde(default)]
        content: Vec<Span>,
    },
    NumberedItem {
        id: String,
        #[serde(default, skip_serializing_if = "is_zero")]
        indent: u8,
        #[serde(default)]
        content: Vec<Span>,
    },
    Todo {
        id: String,
        #[serde(default)]
        checked: bool,
        #[serde(default, skip_serializing_if = "is_zero")]
        indent: u8,
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
        /// Responsive renditions from the ingest pipeline. Empty for a pasted
        /// external URL (and old documents) → the renderer emits a plain `<img>`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        variants: Vec<ImageVariant>,
        /// Blur-up placeholder (`data:image/png;base64,…`); empty when unknown.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        placeholder: String,
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        width: u32,
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        height: u32,
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
