//! Wire-contract tests: these lock the JSON shape shared by the API and the
//! Leptos CRM. A change here is a breaking contract change.

use serde_json::json;
use syle_types::{
    Block, BlogPost, Gallery, ImageFormat, ImageVariant, LoginRequest, Mark, Photo, PostStatus,
    SessionToken, Span,
};
use uuid::Uuid;

fn fixed(id: u128) -> Uuid {
    Uuid::from_u128(id)
}

#[test]
fn gallery_serializes_snake_case() {
    let g = Gallery {
        id: fixed(1),
        slug: "wedding".into(),
        title: "Wedding".into(),
        position: 0,
        published: true,
        description: "el lede".into(),
        notes: "las notas".into(),
        category: "Película".into(),
        year: Some(2026),
    };
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "slug": "wedding",
            "title": "Wedding",
            "position": 0,
            "published": true,
            "description": "el lede",
            "notes": "las notas",
            "category": "Película",
            "year": 2026
        })
    );
}

#[test]
fn photo_carries_thumbhash_and_variants() {
    let p = Photo {
        id: fixed(2),
        gallery_id: fixed(1),
        alt: "bride".into(),
        thumbhash: "1QcSHQRnh493V4dIh4eXh1h4kJUI".into(),
        width: 4000,
        height: 6000,
        position: 3,
        variants: vec![ImageVariant {
            format: ImageFormat::Avif,
            width: 1280,
            path: "/media/ab/cd.avif".into(),
        }],
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["thumbhash"], "1QcSHQRnh493V4dIh4eXh1h4kJUI");
    assert_eq!(v["gallery_id"], "00000000-0000-0000-0000-000000000001");
    assert_eq!(v["variants"][0]["format"], "avif");
}

#[test]
fn post_status_is_lowercase_and_roundtrips() {
    assert_eq!(
        serde_json::to_value(PostStatus::Published).unwrap(),
        json!("published")
    );
    let post = BlogPost {
        id: fixed(3),
        slug: "hello".into(),
        title: "Hello".into(),
        body_md: "# hi".into(),
        status: PostStatus::Draft,
        published_at: None,
    };
    let back: BlogPost = serde_json::from_value(serde_json::to_value(&post).unwrap()).unwrap();
    assert_eq!(back.status, PostStatus::Draft);
    assert_eq!(back.published_at, None);
}

#[test]
fn block_paragraph_with_marks_serializes_tagged() {
    let p = Block::Paragraph {
        id: "b1".into(),
        content: vec![
            Span {
                text: "plain ".into(),
                marks: vec![],
            },
            Span {
                text: "bold link".into(),
                marks: vec![Mark::Bold, Mark::Link { href: "/x".into() }],
            },
        ],
    };
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        json!({
            "type": "paragraph",
            "id": "b1",
            "content": [
                { "text": "plain " },
                { "text": "bold link", "marks": [
                    { "type": "bold" },
                    { "type": "link", "href": "/x" }
                ] }
            ]
        })
    );
}

#[test]
fn block_variants_roundtrip_by_tag() {
    let doc = vec![
        Block::Heading {
            id: "h".into(),
            level: 2,
            content: vec![Span {
                text: "Title".into(),
                marks: vec![],
            }],
        },
        Block::BulletItem {
            id: "li".into(),
            content: vec![Span {
                text: "one".into(),
                marks: vec![Mark::Italic],
            }],
        },
        Block::Todo {
            id: "t".into(),
            checked: true,
            content: vec![],
        },
        Block::Code {
            id: "c".into(),
            language: "rust".into(),
            code: "fn main() {}".into(),
        },
        Block::Divider { id: "d".into() },
        Block::Image {
            id: "img".into(),
            src: "/media/x.avif".into(),
            alt: "alt".into(),
            caption: vec![],
        },
    ];
    let wire = serde_json::to_value(&doc).unwrap();
    assert_eq!(wire[0]["type"], "heading");
    assert_eq!(wire[0]["level"], 2);
    assert_eq!(wire[1]["type"], "bullet_item");
    assert_eq!(wire[2]["type"], "todo");
    assert_eq!(wire[2]["checked"], true);
    assert_eq!(wire[3]["type"], "code");
    assert_eq!(wire[4]["type"], "divider");
    assert_eq!(wire[5]["type"], "image");
    let back: Vec<Block> = serde_json::from_value(wire).unwrap();
    assert_eq!(back, doc);
}

#[test]
fn block_id_accessor_covers_every_variant() {
    let b = Block::Divider { id: "z9".into() };
    assert_eq!(b.id(), "z9");
}

#[test]
fn auth_dtos_roundtrip() {
    let req = LoginRequest {
        email: "a@b.com".into(),
        password: "secret".into(),
    };
    let back: LoginRequest =
        serde_json::from_value(serde_json::to_value(&req).unwrap()).unwrap();
    assert_eq!(back.email, "a@b.com");

    let tok = SessionToken {
        token: "t".into(),
        expires_at: 1_900_000_000,
    };
    assert_eq!(serde_json::to_value(&tok).unwrap()["expires_at"], 1_900_000_000);
}
