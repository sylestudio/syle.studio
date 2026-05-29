//! Pure block → HTML renderer shared by the API (host) and the CRM (wasm).
//!
//! Output is safe by construction: text is HTML-escaped, only a known set of
//! inline marks is emitted, and URLs are scheme-checked — so no external
//! sanitizer is needed. Because both the public site and the CRM preview call
//! this exact function, what the author sees in the CRM is what ships.

use syle_types::{Block, Mark, Span};

/// Render a post document to an HTML fragment.
pub fn render_blocks(blocks: &[Block]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < blocks.len() {
        // Consecutive list items of the same kind collapse into one list.
        if let Some(tag) = list_tag(&blocks[i]) {
            let kind = std::mem::discriminant(&blocks[i]);
            out.push_str(tag.open);
            while i < blocks.len() && std::mem::discriminant(&blocks[i]) == kind {
                push_list_item(&mut out, &blocks[i]);
                i += 1;
            }
            out.push_str(tag.close);
            continue;
        }
        render_block(&mut out, &blocks[i]);
        i += 1;
    }
    out
}

struct ListTag {
    open: &'static str,
    close: &'static str,
}

fn list_tag(b: &Block) -> Option<ListTag> {
    match b {
        Block::BulletItem { .. } => Some(ListTag {
            open: "<ul>",
            close: "</ul>",
        }),
        Block::NumberedItem { .. } => Some(ListTag {
            open: "<ol>",
            close: "</ol>",
        }),
        Block::Todo { .. } => Some(ListTag {
            open: "<ul class=\"todo-list\">",
            close: "</ul>",
        }),
        _ => None,
    }
}

fn push_list_item(out: &mut String, b: &Block) {
    match b {
        Block::BulletItem { content, .. } | Block::NumberedItem { content, .. } => {
            out.push_str("<li>");
            out.push_str(&render_inline(content));
            out.push_str("</li>");
        }
        Block::Todo {
            checked, content, ..
        } => {
            out.push_str("<li class=\"todo\"><input type=\"checkbox\"");
            if *checked {
                out.push_str(" checked");
            }
            out.push_str(" disabled> ");
            out.push_str(&render_inline(content));
            out.push_str("</li>");
        }
        _ => {}
    }
}

fn render_block(out: &mut String, b: &Block) {
    match b {
        Block::Paragraph { content, .. } => {
            out.push_str("<p>");
            out.push_str(&render_inline(content));
            out.push_str("</p>");
        }
        Block::Heading { level, content, .. } => {
            let lvl = (*level).clamp(1, 3);
            out.push_str(&format!("<h{lvl}>"));
            out.push_str(&render_inline(content));
            out.push_str(&format!("</h{lvl}>"));
        }
        Block::Quote { content, .. } => {
            out.push_str("<blockquote><p>");
            out.push_str(&render_inline(content));
            out.push_str("</p></blockquote>");
        }
        Block::Code { language, code, .. } => {
            out.push_str("<pre><code");
            if !language.is_empty() {
                out.push_str(&format!(" class=\"language-{}\"", escape_attr(language)));
            }
            out.push('>');
            out.push_str(&escape_text(code));
            out.push_str("</code></pre>");
        }
        Block::Divider { .. } => out.push_str("<hr>"),
        Block::Image {
            src, alt, caption, ..
        } => {
            if let Some(safe) = sanitize_url(src) {
                out.push_str("<figure><img src=\"");
                out.push_str(&escape_attr(&safe));
                out.push_str("\" alt=\"");
                out.push_str(&escape_attr(alt));
                out.push_str("\">");
                if !caption.is_empty() {
                    out.push_str("<figcaption>");
                    out.push_str(&render_inline(caption));
                    out.push_str("</figcaption>");
                }
                out.push_str("</figure>");
            }
        }
        Block::Callout { emoji, content, .. } => {
            out.push_str("<aside class=\"callout\"><span class=\"callout-emoji\">");
            out.push_str(&escape_text(emoji));
            out.push_str("</span><div>");
            out.push_str(&render_inline(content));
            out.push_str("</div></aside>");
        }
        // List items are handled by the grouping pass; a stray one renders bare.
        Block::BulletItem { content, .. } | Block::NumberedItem { content, .. } => {
            out.push_str("<ul><li>");
            out.push_str(&render_inline(content));
            out.push_str("</li></ul>");
        }
        Block::Todo { .. } => push_list_item(out, b),
    }
}

/// Render inline content (no block wrapper). Used to seed editable blocks in
/// the CRM so the editing surface matches the canonical output exactly.
pub fn render_inline(spans: &[Span]) -> String {
    let mut s = String::new();
    for span in spans {
        s.push_str(&wrap_marks(escape_text(&span.text), &span.marks));
    }
    s
}

/// Wrap escaped text in its marks with a fixed nesting: link outermost, then
/// bold, italic, strike, code innermost. Stable regardless of stored order.
fn wrap_marks(inner: String, marks: &[Mark]) -> String {
    let has = |pred: &dyn Fn(&Mark) -> bool| marks.iter().any(|m| pred(m));
    let mut s = inner;
    if has(&|m| matches!(m, Mark::Code)) {
        s = format!("<code>{s}</code>");
    }
    if has(&|m| matches!(m, Mark::Strike)) {
        s = format!("<s>{s}</s>");
    }
    if has(&|m| matches!(m, Mark::Italic)) {
        s = format!("<em>{s}</em>");
    }
    if has(&|m| matches!(m, Mark::Bold)) {
        s = format!("<strong>{s}</strong>");
    }
    if let Some(Mark::Link { href }) = marks.iter().find(|m| matches!(m, Mark::Link { .. })) {
        if let Some(safe) = sanitize_url(href) {
            s = format!("<a href=\"{}\">{s}</a>", escape_attr(&safe));
        }
    }
    s
}

/// Allow only safe URL shapes; reject `javascript:`, `data:`, etc.
fn sanitize_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let ok = lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || trimmed.starts_with('/')
        || trimmed.starts_with('#');
    ok.then(|| trimmed.to_string())
}

fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_attr(s: &str) -> String {
    // Same set plus single quote; attributes here are always double-quoted.
    let mut out = escape_text(s);
    if out.contains('\'') {
        out = out.replace('\'', "&#39;");
    }
    out
}
