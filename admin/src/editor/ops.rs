//! Structural edits on the document model — convert, split (Enter), merge
//! (Backspace), reorder (drag), indent (Tab), and delete. These mutate the
//! `Vec<Block>` signal and, where a row doesn't re-mount, re-seed its DOM and
//! place the caret. Kept separate from the row view so each stays focused.

use super::content::{self, Kind};
use super::dom;
use leptos::prelude::*;
use syle_render::render_inline;
use syle_types::{Block, Span};
use web_sys::Element;

pub(super) type Blocks = RwSignal<Vec<Block>>;
pub(super) type Slash = RwSignal<Option<String>>;
pub(super) type SlashQuery = RwSignal<String>;
pub(super) type Dragging = RwSignal<Option<String>>;

fn set_block_spans(blocks: Blocks, id: &str, spans: Vec<Span>) {
    blocks.update(|v| {
        if let Some(b) = v.iter_mut().find(|b| b.id() == id) {
            content::set_spans(b, spans);
        }
    });
}

/// Re-seed the block's DOM from the model and place the caret, after render.
fn reseed_focus(blocks: Blocks, id: String, at_start: bool) {
    dom::after_render(move || {
        if let Some(html) = blocks
            .get_untracked()
            .iter()
            .find(|b| b.id() == id)
            .map(|b| render_inline(&content::spans_of(b)))
        {
            dom::reseed_block(&id, &html);
        }
        dom::focus_block(&id, at_start);
    });
}

/// Convert block `id` to `kind`, clearing its content. Non-text blocks
/// (divider/image) get a trailing paragraph to land the caret. Closes the slash
/// menu.
pub(super) fn apply_convert(blocks: Blocks, slash: Slash, id: String, kind: Kind) {
    let para_id = dom::new_id();
    let needs_trailer = matches!(kind, Kind::Divider | Kind::Image);
    let para_for_update = para_id.clone();
    blocks.update(|v| {
        if let Some(i) = v.iter().position(|b| b.id() == id) {
            let bid = v[i].id().to_string();
            v[i] = content::make(kind, bid, Vec::new());
            if needs_trailer {
                v.insert(
                    i + 1,
                    Block::Paragraph {
                        id: para_for_update.clone(),
                        content: Vec::new(),
                    },
                );
            }
        }
    });
    slash.set(None);
    reseed_focus(blocks, if needs_trailer { para_id } else { id }, true);
}

/// Move the currently-dragged block to just before `target_id` (drop target).
pub(super) fn reorder(blocks: Blocks, dragging: Dragging, target_id: &str) {
    let Some(src) = dragging.get_untracked() else {
        return;
    };
    dragging.set(None);
    if src == target_id {
        return;
    }
    blocks.update(|v| {
        let Some(si) = v.iter().position(|b| b.id() == src) else {
            return;
        };
        let b = v.remove(si);
        match v.iter().position(|b| b.id() == target_id) {
            Some(ti) => v.insert(ti, b),
            None => v.insert(si.min(v.len()), b),
        }
    });
}

/// Change a list item's nesting one level (`deeper` = Tab, else Shift+Tab).
/// Indenting is capped at one level past the preceding list item (so the first
/// item of a run can't orphan-indent) and at `MAX_INDENT`. Only the model's
/// `indent` changes — the row's margin reacts to it, so the caret is untouched.
pub(super) fn indent_block(blocks: Blocks, id: &str, deeper: bool) {
    blocks.update(|v| {
        let Some(i) = v.iter().position(|b| b.id() == id) else {
            return;
        };
        let cur = content::indent_of(&v[i]);
        let new = if deeper {
            let max_allowed = if i > 0 && content::is_list(&v[i - 1]) {
                content::indent_of(&v[i - 1]) + 1
            } else {
                0
            };
            (cur + 1).min(max_allowed).min(content::MAX_INDENT)
        } else {
            cur.saturating_sub(1)
        };
        content::set_indent(&mut v[i], new);
    });
}

pub(super) fn handle_input(blocks: Blocks, slash: Slash, query: SlashQuery, id: &str, el: &Element) {
    // contenteditable renders a trailing space as a non-breaking space; fold it
    // back so Markdown triggers like "## " and "- " match.
    let text = el.text_content().unwrap_or_default().replace('\u{00A0}', " ");
    if let Some(kind) = content::detect_shortcut(&text) {
        apply_convert(blocks, slash, id.to_string(), kind);
        return;
    }
    if let Some(rest) = text.strip_prefix('/') {
        // Open once on the first `/`; later keystrokes only refine the query so
        // the menu refilters in place instead of re-mounting (and losing state).
        if slash.get_untracked().as_deref() != Some(id) {
            slash.set(Some(id.to_string()));
        }
        query.set(rest.to_string());
    } else if slash.get_untracked().as_deref() == Some(id) {
        slash.set(None);
    }
    set_block_spans(blocks, id, dom::serialize_inline(el));
}

pub(super) fn handle_enter(blocks: Blocks, slash: Slash, id: &str, kind: Kind, el: &Element) {
    let tail = dom::split_off_tail(el).unwrap_or_default();
    let head = dom::serialize_inline(el);
    let list = matches!(kind, Kind::Bullet | Kind::Numbered | Kind::Todo);
    if list && head.is_empty() && tail.is_empty() {
        // Enter on an empty list item outdents one level, or — at the top
        // level — drops out of the list into a paragraph.
        let indented = blocks.with_untracked(|v| {
            v.iter()
                .find(|b| b.id() == id)
                .map(|b| content::indent_of(b) > 0)
                .unwrap_or(false)
        });
        if indented {
            indent_block(blocks, id, false);
        } else {
            apply_convert(blocks, slash, id.to_string(), Kind::Paragraph);
        }
        return;
    }
    // A split list item keeps the original's nesting depth.
    let indent = blocks.with_untracked(|v| {
        v.iter()
            .find(|b| b.id() == id)
            .map(content::indent_of)
            .unwrap_or(0)
    });
    let cont = if list { kind } else { Kind::Paragraph };
    let new_id = dom::new_id();
    let nid = new_id.clone();
    let id = id.to_string();
    blocks.update(|v| {
        if let Some(i) = v.iter().position(|b| b.id() == id) {
            content::set_spans(&mut v[i], head.clone());
            let mut nb = content::make(cont, new_id.clone(), tail.clone());
            if list {
                content::set_indent(&mut nb, indent);
            }
            v.insert(i + 1, nb);
        }
    });
    dom::after_render(move || dom::focus_block(&nid, true));
}

/// Returns true if the keystroke was handled (caller calls preventDefault).
pub(super) fn handle_backspace(blocks: Blocks, id: &str, el: &Element) -> bool {
    if !dom::caret_at_start(el) {
        return false;
    }
    let cur = dom::serialize_inline(el);
    let id = id.to_string();
    let mut focus_prev: Option<(String, u32)> = None;
    let mut converted = false;
    blocks.update(|v| {
        let Some(i) = v.iter().position(|b| b.id() == id) else {
            return;
        };
        if i == 0 {
            if !matches!(v[0], Block::Paragraph { .. }) {
                let bid = v[0].id().to_string();
                v[0] = content::make(Kind::Paragraph, bid, content::spans_of(&v[0]));
                converted = true;
            }
            return;
        }
        if content::is_text(&v[i - 1]) {
            let mut merged = content::spans_of(&v[i - 1]);
            let join = dom::spans_len(&merged);
            merged.extend(cur.clone());
            let prev_id = v[i - 1].id().to_string();
            content::set_spans(&mut v[i - 1], merged);
            v.remove(i);
            focus_prev = Some((prev_id, join));
        } else {
            v.remove(i - 1);
        }
    });
    if let Some((pid, join)) = focus_prev {
        dom::after_render(move || {
            if let Some(html) = blocks
                .get_untracked()
                .iter()
                .find(|b| b.id() == pid)
                .map(|b| render_inline(&content::spans_of(b)))
            {
                dom::reseed_block(&pid, &html);
            }
            dom::focus_block_offset(&pid, join);
        });
        true
    } else if converted {
        reseed_focus(blocks, id, true);
        true
    } else {
        true // at start with no prior text block: swallow so caret doesn't drift
    }
}

pub(super) fn remove_block(blocks: Blocks, id: &str) {
    blocks.update(|v| {
        v.retain(|b| b.id() != id);
        if v.is_empty() {
            v.push(content::empty_paragraph());
        }
    });
}
