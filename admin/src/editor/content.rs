//! Pure block helpers: inline-content access, type conversion, and the
//! Markdown auto-shortcut table. No DOM, no Leptos — just `Block` transforms.

use super::dom::new_id;
use syle_types::{Block, Span};

/// Block kinds the editor can create or convert between.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Paragraph,
    H1,
    H2,
    H3,
    Bullet,
    Numbered,
    Todo,
    Quote,
    Code,
    Divider,
    Callout,
    Image,
}

impl Kind {
    /// Label shown in the slash menu.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Paragraph => "Texto",
            Kind::H1 => "Título 1",
            Kind::H2 => "Título 2",
            Kind::H3 => "Título 3",
            Kind::Bullet => "Lista con viñetas",
            Kind::Numbered => "Lista numerada",
            Kind::Todo => "Casilla",
            Kind::Quote => "Cita",
            Kind::Code => "Código",
            Kind::Divider => "Divisor",
            Kind::Callout => "Llamado",
            Kind::Image => "Imagen",
        }
    }

    /// Monospace glyph hint for the slash menu.
    pub fn glyph(self) -> &'static str {
        match self {
            Kind::Paragraph => "¶",
            Kind::H1 => "H1",
            Kind::H2 => "H2",
            Kind::H3 => "H3",
            Kind::Bullet => "•",
            Kind::Numbered => "1.",
            Kind::Todo => "☐",
            Kind::Quote => "❝",
            Kind::Code => "</>",
            Kind::Divider => "—",
            Kind::Callout => "💡",
            Kind::Image => "🖼",
        }
    }

    /// Kinds offered by the slash menu, in order.
    pub fn menu() -> &'static [Kind] {
        &[
            Kind::Paragraph,
            Kind::H1,
            Kind::H2,
            Kind::H3,
            Kind::Bullet,
            Kind::Numbered,
            Kind::Todo,
            Kind::Quote,
            Kind::Code,
            Kind::Callout,
            Kind::Image,
            Kind::Divider,
        ]
    }
}

/// Slash-menu kinds matching `query` (the text typed after `/`), by label or
/// glyph substring. Empty query → the full menu, in order.
pub fn filter_kinds(query: &str) -> Vec<Kind> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Kind::menu().to_vec();
    }
    Kind::menu()
        .iter()
        .copied()
        .filter(|k| k.label().to_lowercase().contains(&q) || k.glyph().to_lowercase().contains(&q))
        .collect()
}

/// True for blocks edited through a contenteditable surface.
pub fn is_text(b: &Block) -> bool {
    matches!(
        b,
        Block::Paragraph { .. }
            | Block::Heading { .. }
            | Block::Quote { .. }
            | Block::BulletItem { .. }
            | Block::NumberedItem { .. }
            | Block::Todo { .. }
            | Block::Callout { .. }
    )
}

/// True when `b` is a text block whose inline content is empty (no spans, or
/// only empty-text spans). Drives the focused-block placeholder hint.
pub fn is_empty_text(b: &Block) -> bool {
    match b {
        Block::Paragraph { content, .. }
        | Block::Heading { content, .. }
        | Block::Quote { content, .. }
        | Block::BulletItem { content, .. }
        | Block::NumberedItem { content, .. }
        | Block::Todo { content, .. }
        | Block::Callout { content, .. } => content.iter().all(|s| s.text.is_empty()),
        _ => false,
    }
}

/// Inline content of a text block (empty for non-text blocks).
pub fn spans_of(b: &Block) -> Vec<Span> {
    match b {
        Block::Paragraph { content, .. }
        | Block::Heading { content, .. }
        | Block::Quote { content, .. }
        | Block::BulletItem { content, .. }
        | Block::NumberedItem { content, .. }
        | Block::Todo { content, .. }
        | Block::Callout { content, .. } => content.clone(),
        _ => Vec::new(),
    }
}

/// Replace a text block's inline content in place.
pub fn set_spans(b: &mut Block, spans: Vec<Span>) {
    match b {
        Block::Paragraph { content, .. }
        | Block::Heading { content, .. }
        | Block::Quote { content, .. }
        | Block::BulletItem { content, .. }
        | Block::NumberedItem { content, .. }
        | Block::Todo { content, .. }
        | Block::Callout { content, .. } => *content = spans,
        _ => {}
    }
}

/// Build a block of `kind` with the given id and inline content.
pub fn make(kind: Kind, id: String, spans: Vec<Span>) -> Block {
    match kind {
        Kind::Paragraph => Block::Paragraph { id, content: spans },
        Kind::H1 => Block::Heading {
            id,
            level: 1,
            content: spans,
        },
        Kind::H2 => Block::Heading {
            id,
            level: 2,
            content: spans,
        },
        Kind::H3 => Block::Heading {
            id,
            level: 3,
            content: spans,
        },
        Kind::Bullet => Block::BulletItem { id, content: spans },
        Kind::Numbered => Block::NumberedItem { id, content: spans },
        Kind::Todo => Block::Todo {
            id,
            checked: false,
            content: spans,
        },
        Kind::Quote => Block::Quote { id, content: spans },
        Kind::Callout => Block::Callout {
            id,
            emoji: "💡".into(),
            content: spans,
        },
        Kind::Code => Block::Code {
            id,
            language: String::new(),
            code: spans.iter().map(|s| s.text.as_str()).collect(),
        },
        Kind::Divider => Block::Divider { id },
        Kind::Image => Block::Image {
            id,
            src: String::new(),
            alt: String::new(),
            caption: Vec::new(),
        },
    }
}

/// The kind of an existing block (Image falls back to Paragraph; it is handled
/// by variant, not kind, in the row view).
pub fn kind_of(b: &Block) -> Kind {
    match b {
        Block::Paragraph { .. } => Kind::Paragraph,
        Block::Heading { level: 1, .. } => Kind::H1,
        Block::Heading { level: 2, .. } => Kind::H2,
        Block::Heading { .. } => Kind::H3,
        Block::Quote { .. } => Kind::Quote,
        Block::BulletItem { .. } => Kind::Bullet,
        Block::NumberedItem { .. } => Kind::Numbered,
        Block::Todo { .. } => Kind::Todo,
        Block::Code { .. } => Kind::Code,
        Block::Divider { .. } => Kind::Divider,
        Block::Callout { .. } => Kind::Callout,
        Block::Image { .. } => Kind::Image,
    }
}

/// A fresh empty paragraph (new id).
pub fn empty_paragraph() -> Block {
    Block::Paragraph {
        id: new_id(),
        content: Vec::new(),
    }
}

/// Re-render key: changes only on a *structural* edit (kind/level/checked/img),
/// never on a content edit — so typing never re-mounts a row (caret-safe) but a
/// type conversion does.
pub fn signature(b: &Block) -> String {
    let id = b.id();
    match b {
        Block::Paragraph { .. } => format!("{id}|p"),
        Block::Heading { level, .. } => format!("{id}|h{level}"),
        Block::Quote { .. } => format!("{id}|q"),
        Block::BulletItem { .. } => format!("{id}|ul"),
        Block::NumberedItem { .. } => format!("{id}|ol"),
        Block::Todo { checked, .. } => format!("{id}|td{checked}"),
        Block::Code { .. } => format!("{id}|code"),
        Block::Divider { .. } => format!("{id}|hr"),
        Block::Image { src, .. } => format!("{id}|img{src}"),
        Block::Callout { .. } => format!("{id}|co"),
    }
}

/// Detect a leading Markdown shortcut once the trigger text is fully typed.
/// Returns the target kind; the caller clears the block's content.
pub fn detect_shortcut(text: &str) -> Option<Kind> {
    match text {
        "# " => Some(Kind::H1),
        "## " => Some(Kind::H2),
        "### " => Some(Kind::H3),
        "- " | "* " => Some(Kind::Bullet),
        "1. " => Some(Kind::Numbered),
        "> " => Some(Kind::Quote),
        "[] " | "[ ] " => Some(Kind::Todo),
        "```" => Some(Kind::Code),
        "---" | "***" => Some(Kind::Divider),
        _ => None,
    }
}
