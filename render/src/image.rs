//! Image-block rendering. An uploaded image carries responsive renditions
//! (AVIF+JPEG `srcset`) and a blur-up placeholder, so it renders as a
//! `<picture>` mirroring the gallery `Photo` component — same optimization,
//! identical on the public site and the CRM preview. A pasted external URL (or
//! an old document) has no renditions and falls back to a plain `<img>`.

use crate::{escape_attr, render_inline, sanitize_url};
use syle_types::{ImageFormat, ImageVariant, Span};

/// `sizes` hint: the post body is a single ~44rem column.
const SIZES: &str = "(min-width: 44rem) 44rem, 100vw";

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_image(
    out: &mut String,
    src: &str,
    alt: &str,
    caption: &[Span],
    variants: &[ImageVariant],
    placeholder: &str,
    width: u32,
    height: u32,
) {
    // The fallback src is shared by both branches; an unsafe URL drops the whole
    // figure (same guard as a bare external URL).
    let Some(safe) = sanitize_url(src) else {
        return;
    };
    out.push_str("<figure>");
    if variants.is_empty() {
        out.push_str("<img src=\"");
        out.push_str(&escape_attr(&safe));
        out.push_str("\" alt=\"");
        out.push_str(&escape_attr(alt));
        out.push_str("\">");
    } else {
        out.push_str("<picture>");
        push_source(out, variants, ImageFormat::Avif, "image/avif");
        push_source(out, variants, ImageFormat::Jpeg, "image/jpeg");
        out.push_str("<img src=\"");
        out.push_str(&escape_attr(&safe));
        out.push_str("\" alt=\"");
        out.push_str(&escape_attr(alt));
        out.push('"');
        if width > 0 && height > 0 {
            out.push_str(&format!(" width=\"{width}\" height=\"{height}\""));
        }
        out.push_str(" loading=\"lazy\" decoding=\"async\"");
        // Only emit a placeholder we produced (a PNG data URL); never reflect
        // arbitrary stored text into a style attribute unchecked.
        if placeholder.starts_with("data:image/") {
            out.push_str(" style=\"background-image:url(");
            out.push_str(&escape_attr(placeholder));
            out.push_str(");background-size:cover\"");
        }
        out.push('>');
        out.push_str("</picture>");
    }
    if !caption.is_empty() {
        out.push_str("<figcaption>");
        out.push_str(&render_inline(caption));
        out.push_str("</figcaption>");
    }
    out.push_str("</figure>");
}

/// Emit one `<source>` for a format, sorted ascending by width, or nothing when
/// that format has no renditions.
fn push_source(out: &mut String, variants: &[ImageVariant], fmt: ImageFormat, mime: &str) {
    let mut vs: Vec<&ImageVariant> = variants.iter().filter(|v| v.format == fmt).collect();
    if vs.is_empty() {
        return;
    }
    vs.sort_by_key(|v| v.width);
    let srcset = vs
        .iter()
        .map(|v| format!("{} {}w", escape_attr(&v.path), v.width))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str("<source srcset=\"");
    out.push_str(&srcset);
    out.push_str("\" sizes=\"");
    out.push_str(SIZES);
    out.push_str("\" type=\"");
    out.push_str(mime);
    out.push_str("\">");
}
