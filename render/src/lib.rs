//! Pure block → HTML renderer shared by the API (host) and the CRM (wasm).
//!
//! Output is safe by construction: text is HTML-escaped, only a known set of
//! inline marks is emitted, and URLs are scheme-checked — so no external
//! sanitizer is needed. Because both the public site and the CRM preview call
//! this exact function, what the author sees in the CRM is what ships.

use syle_types::{Block, Mark, Span};

mod highlight;
mod image;
mod list;

/// Render a post document to an HTML fragment.
pub fn render_blocks(blocks: &[Block]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < blocks.len() {
        // A run of consecutive list items becomes one (possibly nested) list.
        if list::is_list_item(&blocks[i]) {
            let start = i;
            while i < blocks.len() && list::is_list_item(&blocks[i]) {
                i += 1;
            }
            list::render_list_run(&mut out, &blocks[start..i]);
            continue;
        }
        render_block(&mut out, &blocks[i]);
        i += 1;
    }
    out
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
            out.push_str(&highlight::highlight(language, code));
            out.push_str("</code></pre>");
        }
        Block::Divider { .. } => out.push_str("<hr>"),
        Block::Image {
            src,
            alt,
            caption,
            variants,
            placeholder,
            width,
            height,
            ..
        } => image::render_image(out, src, alt, caption, variants, placeholder, *width, *height),
        Block::Callout { emoji, content, .. } => {
            out.push_str("<aside class=\"callout\"><span class=\"callout-emoji\">");
            out.push_str(&escape_text(emoji));
            out.push_str("</span><div>");
            out.push_str(&render_inline(content));
            out.push_str("</div></aside>");
        }
        // List items are normally consumed as a run by `render_blocks`; a stray
        // one (e.g. a single item) still renders as a one-item list.
        Block::BulletItem { .. } | Block::NumberedItem { .. } | Block::Todo { .. } => {
            list::render_list_run(out, std::slice::from_ref(b));
        }
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
    let has = |pred: &dyn Fn(&Mark) -> bool| marks.iter().any(pred);
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

/// Normalize a link target typed by an author into a storable href, or `None`
/// when it can't be made safe. Acceptable shapes (`http(s)://`, `mailto:`,
/// `/path`, `#anchor`) pass through the render gate unchanged; a scheme-less
/// host like `example.com` gains an `https://` prefix (otherwise the gate would
/// later drop it silently, leaving the text un-linked); explicit dangerous or
/// unknown schemes (`javascript:`, `data:`, `ftp://`, …) are rejected.
pub fn coerce_href(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || trimmed.starts_with('/')
        || trimmed.starts_with('#')
    {
        return sanitize_url(trimmed);
    }
    // Any other explicit scheme (a `scheme://` authority or a dangerous prefix)
    // is rejected rather than coerced.
    if lower.contains("://") || is_dangerous_scheme(&lower) {
        return None;
    }
    // Scheme-less, non-relative → assume https.
    sanitize_url(&format!("https://{trimmed}"))
}

fn is_dangerous_scheme(lower: &str) -> bool {
    ["javascript:", "data:", "vbscript:", "file:"]
        .iter()
        .any(|p| lower.starts_with(p))
}

/// Allow only safe URL shapes; reject `javascript:`, `data:`, etc.
pub(crate) fn sanitize_url(url: &str) -> Option<String> {
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

pub(crate) fn escape_text(s: &str) -> String {
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

pub(crate) fn escape_attr(s: &str) -> String {
    // Same set plus single quote; attributes here are always double-quoted.
    let mut out = escape_text(s);
    if out.contains('\'') {
        out = out.replace('\'', "&#39;");
    }
    out
}
