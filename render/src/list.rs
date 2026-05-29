//! Nested-list rendering. A run of consecutive list-item blocks
//! (`BulletItem`/`NumberedItem`/`Todo`) is turned into nested `<ul>`/`<ol>` by
//! each item's `indent`. The child list nests *inside* the parent `<li>`, so
//! the `<li>` is left open until its descendants close. Indents are normalized
//! to rise by at most one level per item, keeping the close stack and the input
//! in lockstep even if stored data has a malformed jump.

use crate::render_inline;
use syle_types::Block;

#[derive(Clone, Copy, PartialEq)]
enum ListKind {
    Bullet,
    Numbered,
    Todo,
}

impl ListKind {
    fn open(self) -> &'static str {
        match self {
            ListKind::Bullet => "<ul>",
            ListKind::Numbered => "<ol>",
            ListKind::Todo => "<ul class=\"todo-list\">",
        }
    }
    fn close(self) -> &'static str {
        match self {
            ListKind::Numbered => "</ol>",
            _ => "</ul>",
        }
    }
}

fn list_kind(b: &Block) -> Option<ListKind> {
    match b {
        Block::BulletItem { .. } => Some(ListKind::Bullet),
        Block::NumberedItem { .. } => Some(ListKind::Numbered),
        Block::Todo { .. } => Some(ListKind::Todo),
        _ => None,
    }
}

/// True when `b` is a list item (the run boundary `render_blocks` groups on).
pub(crate) fn is_list_item(b: &Block) -> bool {
    list_kind(b).is_some()
}

fn indent_of(b: &Block) -> u8 {
    match b {
        Block::BulletItem { indent, .. }
        | Block::NumberedItem { indent, .. }
        | Block::Todo { indent, .. } => *indent,
        _ => 0,
    }
}

/// Emit one item's opening `<li …>` and inline content, leaving the `<li>` open
/// so a nested child list can be appended before it is closed.
fn open_item(out: &mut String, b: &Block) {
    match b {
        Block::Todo {
            checked, content, ..
        } => {
            out.push_str("<li class=\"todo\"><input type=\"checkbox\"");
            if *checked {
                out.push_str(" checked");
            }
            out.push_str(" disabled> ");
            out.push_str(&render_inline(content));
        }
        Block::BulletItem { content, .. } | Block::NumberedItem { content, .. } => {
            out.push_str("<li>");
            out.push_str(&render_inline(content));
        }
        _ => {}
    }
}

/// Render a run of consecutive list-item blocks into (possibly nested) lists.
pub(crate) fn render_list_run(out: &mut String, items: &[Block]) {
    // (level, kind) for each open list, outermost first.
    let mut stack: Vec<(u8, ListKind)> = Vec::new();
    let mut prev_level = 0u8;
    for (i, item) in items.iter().enumerate() {
        let Some(kind) = list_kind(item) else { continue };
        // The first item is top level; otherwise a level may rise by at most one.
        let level = if i == 0 {
            0
        } else {
            indent_of(item).min(prev_level + 1)
        };

        if stack.last().is_none_or(|&(l, _)| level > l) {
            // Deeper (or the first list): open a nested list in the open <li>.
            out.push_str(kind.open());
            stack.push((level, kind));
        } else {
            // Same level or shallower: close lists down to `level`…
            while matches!(stack.last(), Some(&(l, _)) if l > level) {
                let (_, k) = stack.pop().unwrap();
                out.push_str("</li>");
                out.push_str(k.close());
            }
            // …then close the sibling <li>.
            out.push_str("</li>");
            // A different kind at this level becomes a fresh sibling list.
            if matches!(stack.last(), Some(&(_, k)) if k != kind) {
                let (_, k) = stack.pop().unwrap();
                out.push_str(k.close());
                out.push_str(kind.open());
                stack.push((level, kind));
            }
        }
        open_item(out, item);
        prev_level = level;
    }
    while let Some((_, kind)) = stack.pop() {
        out.push_str("</li>");
        out.push_str(kind.close());
    }
}
