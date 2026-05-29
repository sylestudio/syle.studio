use syle_render::render_blocks;
use syle_types::{Block, Mark, Span};

fn para(id: &str, spans: Vec<Span>) -> Block {
    Block::Paragraph {
        id: id.into(),
        content: spans,
    }
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
            content: vec![Span::plain("one")],
        },
        Block::BulletItem {
            id: "b".into(),
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
            content: vec![Span::plain("one")],
        },
        Block::NumberedItem {
            id: "b".into(),
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
            content: vec![Span::plain("done")],
        },
        Block::Todo {
            id: "b".into(),
            checked: false,
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
fn code_block_escapes_and_carries_language() {
    let html = render_blocks(&[Block::Code {
        id: "c".into(),
        language: "rust".into(),
        code: "fn main() { let _ = 1 < 2; }".into(),
    }]);
    assert_eq!(
        html,
        "<pre><code class=\"language-rust\">fn main() { let _ = 1 &lt; 2; }</code></pre>"
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
fn image_with_caption_renders_figure() {
    let html = render_blocks(&[Block::Image {
        id: "i".into(),
        src: "/media/ab/cd.avif".into(),
        alt: "a cat".into(),
        caption: vec![Span::plain("kitty")],
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
    }]);
    assert_eq!(html, "");
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
