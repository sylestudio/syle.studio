use syle_render::render_blocks;
use syle_types::{Block, ImageFormat, ImageVariant, Mark, Span};

fn para(id: &str, spans: Vec<Span>) -> Block {
    Block::Paragraph {
        id: id.into(),
        content: spans,
    }
}

fn bullet(id: &str, indent: u8, text: &str) -> Block {
    Block::BulletItem {
        id: id.into(),
        indent,
        content: vec![Span::plain(text)],
    }
}

fn numbered(id: &str, indent: u8, text: &str) -> Block {
    Block::NumberedItem {
        id: id.into(),
        indent,
        content: vec![Span::plain(text)],
    }
}

fn todo_item(id: &str, indent: u8, checked: bool, text: &str) -> Block {
    Block::Todo {
        id: id.into(),
        indent,
        checked,
        content: vec![Span::plain(text)],
    }
}

#[test]
fn nested_bullets_indent_one_level() {
    let html = render_blocks(&[bullet("a", 0, "one"), bullet("b", 1, "sub"), bullet("c", 0, "two")]);
    assert_eq!(html, "<ul><li>one<ul><li>sub</li></ul></li><li>two</li></ul>");
}

#[test]
fn dedent_more_than_one_level_at_once_closes_all() {
    let html = render_blocks(&[
        bullet("a", 0, "a"),
        bullet("b", 1, "b"),
        bullet("c", 2, "c"),
        bullet("d", 0, "d"),
    ]);
    assert_eq!(
        html,
        "<ul><li>a<ul><li>b<ul><li>c</li></ul></li></ul></li><li>d</li></ul>"
    );
}

#[test]
fn siblings_share_one_nested_list() {
    let html = render_blocks(&[
        bullet("a", 0, "a"),
        bullet("b", 1, "b1"),
        bullet("c", 1, "b2"),
        bullet("d", 0, "c"),
    ]);
    assert_eq!(
        html,
        "<ul><li>a<ul><li>b1</li><li>b2</li></ul></li><li>c</li></ul>"
    );
}

#[test]
fn mixed_list_kinds_nest_by_indent() {
    let html = render_blocks(&[
        numbered("a", 0, "step"),
        bullet("b", 1, "note"),
        numbered("c", 0, "step2"),
    ]);
    assert_eq!(
        html,
        "<ol><li>step<ul><li>note</li></ul></li><li>step2</li></ol>"
    );
}

#[test]
fn list_kind_change_at_same_level_splits_lists() {
    let html = render_blocks(&[bullet("a", 0, "x"), numbered("b", 0, "y")]);
    assert_eq!(html, "<ul><li>x</li></ul><ol><li>y</li></ol>");
}

#[test]
fn todos_nest_under_a_bullet_keeping_checkbox_state() {
    let html = render_blocks(&[
        bullet("a", 0, "tasks"),
        todo_item("b", 1, true, "done"),
        todo_item("c", 1, false, "pending"),
    ]);
    assert_eq!(
        html,
        "<ul><li>tasks<ul class=\"todo-list\">\
         <li class=\"todo\"><input type=\"checkbox\" checked disabled> done</li>\
         <li class=\"todo\"><input type=\"checkbox\" disabled> pending</li>\
         </ul></li></ul>"
    );
}

#[test]
fn malformed_indent_jump_is_clamped_to_one_level() {
    // The first item can't be nested; a jump from 0 to 3 only nests one level.
    let html = render_blocks(&[bullet("a", 2, "a"), bullet("b", 3, "b")]);
    assert_eq!(html, "<ul><li>a<ul><li>b</li></ul></li></ul>");
}

#[test]
fn paragraph_escapes_text() {
    let html = render_blocks(&[para("p", vec![Span::plain("a < b & \"c\"")])]);
    assert_eq!(html, "<p>a &lt; b &amp; &quot;c&quot;</p>");
}

#[test]
fn marks_nest_with_link_outermost() {
    // Regardless of the order marks are stored, link wraps outside emphasis,
    // emphasis outside code — a fixed, stable nesting.
    let html = render_blocks(&[para(
        "p",
        vec![Span {
            text: "x".into(),
            marks: vec![Mark::Code, Mark::Italic, Mark::Bold, Mark::Link { href: "/y".into() }],
        }],
    )]);
    assert_eq!(
        html,
        "<p><a href=\"/y\"><strong><em><code>x</code></em></strong></a></p>"
    );
}

#[test]
fn inline_code_mark_renders_code_tag() {
    let html = render_blocks(&[para(
        "p",
        vec![Span {
            text: "let x".into(),
            marks: vec![Mark::Code],
        }],
    )]);
    assert_eq!(html, "<p><code>let x</code></p>");
}

#[test]
fn javascript_url_is_dropped_to_plain_text() {
    let html = render_blocks(&[para(
        "p",
        vec![Span {
            text: "click".into(),
            marks: vec![Mark::Link {
                href: "javascript:alert(1)".into(),
            }],
        }],
    )]);
    assert_eq!(html, "<p>click</p>");
}

#[test]
fn heading_level_clamped_to_1_3() {
    let h = |level: u8| {
        render_blocks(&[Block::Heading {
            id: "h".into(),
            level,
            content: vec![Span::plain("T")],
        }])
    };
    assert_eq!(h(1), "<h1>T</h1>");
    assert_eq!(h(3), "<h3>T</h3>");
    assert_eq!(h(9), "<h3>T</h3>");
    assert_eq!(h(0), "<h1>T</h1>");
}

#[test]
fn consecutive_bullets_group_into_one_list() {
    let html = render_blocks(&[
        Block::BulletItem {
            id: "a".into(),
            indent: 0,
            content: vec![Span::plain("one")],
        },
        Block::BulletItem {
            id: "b".into(),
            indent: 0,
            content: vec![Span::plain("two")],
        },
        para("p", vec![Span::plain("after")]),
    ]);
    assert_eq!(
        html,
        "<ul><li>one</li><li>two</li></ul><p>after</p>"
    );
}

#[test]
fn numbered_items_group_into_ordered_list() {
    let html = render_blocks(&[
        Block::NumberedItem {
            id: "a".into(),
            indent: 0,
            content: vec![Span::plain("one")],
        },
        Block::NumberedItem {
            id: "b".into(),
            indent: 0,
            content: vec![Span::plain("two")],
        },
    ]);
    assert_eq!(html, "<ol><li>one</li><li>two</li></ol>");
}

#[test]
fn todos_group_with_checkbox_state() {
    let html = render_blocks(&[
        Block::Todo {
            id: "a".into(),
            checked: true,
            indent: 0,
            content: vec![Span::plain("done")],
        },
        Block::Todo {
            id: "b".into(),
            checked: false,
            indent: 0,
            content: vec![Span::plain("todo")],
        },
    ]);
    assert_eq!(
        html,
        "<ul class=\"todo-list\">\
         <li class=\"todo\"><input type=\"checkbox\" checked disabled> done</li>\
         <li class=\"todo\"><input type=\"checkbox\" disabled> todo</li>\
         </ul>"
    );
}

#[test]
fn code_block_highlights_known_language_and_escapes() {
    let html = render_blocks(&[Block::Code {
        id: "c".into(),
        language: "rust".into(),
        code: "fn main() { let _ = 1 < 2; }".into(),
    }]);
    assert_eq!(
        html,
        "<pre><code class=\"language-rust\">\
         <span class=\"hl-kw\">fn</span> main() { \
         <span class=\"hl-kw\">let</span> _ = \
         <span class=\"hl-num\">1</span> &lt; <span class=\"hl-num\">2</span>; }\
         </code></pre>"
    );
}

#[test]
fn code_block_highlights_strings_and_comments_escaping_within() {
    let html = render_blocks(&[Block::Code {
        id: "c".into(),
        language: "rust".into(),
        code: "let s = \"a<b\"; // c & d".into(),
    }]);
    assert_eq!(
        html,
        "<pre><code class=\"language-rust\">\
         <span class=\"hl-kw\">let</span> s = \
         <span class=\"hl-str\">&quot;a&lt;b&quot;</span>; \
         <span class=\"hl-com\">// c &amp; d</span>\
         </code></pre>"
    );
}

#[test]
fn code_block_unknown_language_is_plain_escaped() {
    let html = render_blocks(&[Block::Code {
        id: "c".into(),
        language: "doesnotexist".into(),
        code: "a < b & c".into(),
    }]);
    assert_eq!(
        html,
        "<pre><code class=\"language-doesnotexist\">a &lt; b &amp; c</code></pre>"
    );
}

#[test]
fn divider_and_quote() {
    let html = render_blocks(&[
        Block::Divider { id: "d".into() },
        Block::Quote {
            id: "q".into(),
            content: vec![Span::plain("said")],
        },
    ]);
    assert_eq!(html, "<hr><blockquote><p>said</p></blockquote>");
}

#[test]
fn image_without_variants_renders_plain_figure() {
    // A pasted external URL (or an old document) has no renditions → plain img.
    let html = render_blocks(&[Block::Image {
        id: "i".into(),
        src: "/media/ab/cd.avif".into(),
        alt: "a cat".into(),
        caption: vec![Span::plain("kitty")],
        variants: vec![],
        placeholder: String::new(),
        width: 0,
        height: 0,
    }]);
    assert_eq!(
        html,
        "<figure><img src=\"/media/ab/cd.avif\" alt=\"a cat\"><figcaption>kitty</figcaption></figure>"
    );
}

#[test]
fn image_with_dangerous_src_is_dropped() {
    let html = render_blocks(&[Block::Image {
        id: "i".into(),
        src: "javascript:alert(1)".into(),
        alt: "x".into(),
        caption: vec![],
        variants: vec![],
        placeholder: String::new(),
        width: 0,
        height: 0,
    }]);
    assert_eq!(html, "");
}

#[test]
fn image_with_variants_renders_responsive_picture() {
    let html = render_blocks(&[Block::Image {
        id: "i".into(),
        src: "/media/jpeg/k_1440.jpeg".into(),
        alt: "foto".into(),
        caption: vec![],
        variants: vec![
            ImageVariant { format: ImageFormat::Jpeg, width: 1440, path: "/media/jpeg/k_1440.jpeg".into() },
            ImageVariant { format: ImageFormat::Avif, width: 480, path: "/media/avif/k_480.avif".into() },
            ImageVariant { format: ImageFormat::Avif, width: 960, path: "/media/avif/k_960.avif".into() },
            ImageVariant { format: ImageFormat::Jpeg, width: 480, path: "/media/jpeg/k_480.jpeg".into() },
        ],
        placeholder: "data:image/png;base64,AAAA".into(),
        width: 1440,
        height: 960,
    }]);
    assert_eq!(
        html,
        "<figure><picture>\
         <source srcset=\"/media/avif/k_480.avif 480w, /media/avif/k_960.avif 960w\" \
         sizes=\"(min-width: 44rem) 44rem, 100vw\" type=\"image/avif\">\
         <source srcset=\"/media/jpeg/k_480.jpeg 480w, /media/jpeg/k_1440.jpeg 1440w\" \
         sizes=\"(min-width: 44rem) 44rem, 100vw\" type=\"image/jpeg\">\
         <img src=\"/media/jpeg/k_1440.jpeg\" alt=\"foto\" width=\"1440\" height=\"960\" \
         loading=\"lazy\" decoding=\"async\" \
         style=\"background-image:url(data:image/png;base64,AAAA);background-size:cover\">\
         </picture></figure>"
    );
}

#[test]
fn image_with_variants_but_no_placeholder_omits_blur_style() {
    let html = render_blocks(&[Block::Image {
        id: "i".into(),
        src: "/media/jpeg/k_960.jpeg".into(),
        alt: "x".into(),
        caption: vec![],
        variants: vec![ImageVariant {
            format: ImageFormat::Jpeg,
            width: 960,
            path: "/media/jpeg/k_960.jpeg".into(),
        }],
        placeholder: String::new(),
        width: 960,
        height: 640,
    }]);
    assert!(html.contains("<picture>"));
    assert!(!html.contains("style="));
    assert!(html.contains("width=\"960\" height=\"640\""));
}

#[test]
fn callout_renders_emoji_and_body() {
    let html = render_blocks(&[Block::Callout {
        id: "c".into(),
        emoji: "💡".into(),
        content: vec![Span::plain("tip")],
    }]);
    assert_eq!(
        html,
        "<aside class=\"callout\"><span class=\"callout-emoji\">💡</span><div>tip</div></aside>"
    );
}

#[test]
fn empty_document_is_empty_string() {
    assert_eq!(render_blocks(&[]), "");
}

#[test]
fn inline_renderer_has_no_block_wrapper() {
    use syle_render::render_inline;
    let html = render_inline(&[
        Span::plain("a "),
        Span {
            text: "b".into(),
            marks: vec![Mark::Bold],
        },
    ]);
    assert_eq!(html, "a <strong>b</strong>");
}

#[test]
fn coerce_href_adds_https_to_bare_domain() {
    use syle_render::coerce_href;
    // A bare domain would otherwise be dropped silently by the render gate.
    assert_eq!(coerce_href("example.com"), Some("https://example.com".into()));
    assert_eq!(
        coerce_href("sub.example.com/path?q=1"),
        Some("https://sub.example.com/path?q=1".into())
    );
}

#[test]
fn coerce_href_preserves_acceptable_shapes() {
    use syle_render::coerce_href;
    assert_eq!(coerce_href("https://x.io"), Some("https://x.io".into()));
    assert_eq!(coerce_href("http://x.io"), Some("http://x.io".into()));
    assert_eq!(coerce_href("/blog/hola"), Some("/blog/hola".into()));
    assert_eq!(coerce_href("#seccion"), Some("#seccion".into()));
    assert_eq!(coerce_href("mailto:a@b.com"), Some("mailto:a@b.com".into()));
    // Surrounding whitespace is trimmed.
    assert_eq!(coerce_href("  example.com  "), Some("https://example.com".into()));
}

#[test]
fn coerce_href_rejects_dangerous_or_empty() {
    use syle_render::coerce_href;
    assert_eq!(coerce_href("javascript:alert(1)"), None);
    assert_eq!(coerce_href("JavaScript:alert(1)"), None);
    assert_eq!(coerce_href("data:text/html,x"), None);
    assert_eq!(coerce_href("ftp://h/f"), None);
    assert_eq!(coerce_href(""), None);
    assert_eq!(coerce_href("   "), None);
}
