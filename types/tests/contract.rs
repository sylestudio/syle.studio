//! Wire-contract tests: these lock the JSON shape shared by the API and the
//! Leptos CRM. A change here is a breaking contract change.

use serde_json::json;
use syle_types::{
    BlogPost, Gallery, ImageFormat, ImageVariant, LoginRequest, Photo, PostStatus, SessionToken,
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
    };
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "slug": "wedding",
            "title": "Wedding",
            "position": 0,
            "published": true
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
