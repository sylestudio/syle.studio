//! API integration tests: public read filtering.

mod common;
use common::*;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use syle_types::{BlogPost, Gallery, GalleryDetail};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[serial_test::serial]
async fn public_galleries_lists_only_published() {
    let Some((app, pool, _media)) = setup().await else { return };
    for (slug, published) in [("shown", true), ("hidden", false)] {
        sqlx::query(
            "INSERT INTO galleries (id, slug, title, position, published) \
             VALUES ($1,$2,$3,0,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(slug)
        .bind(slug)
        .bind(published)
        .execute(&pool)
        .await
        .unwrap();
    }

    let resp = app
        .oneshot(
            Request::get("/api/public/galleries")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let galleries: Vec<Gallery> = body_json(resp).await;
    assert_eq!(galleries.len(), 1);
    assert_eq!(galleries[0].slug, "shown");
}

#[tokio::test]
#[serial_test::serial]
async fn gallery_detail_returns_photos_with_variants() {
    let Some((app, pool, _media)) = setup().await else { return };
    let gid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO galleries \
         (id, slug, title, position, published, description, notes, category, year) \
         VALUES ($1,'wd','Wedding',0,TRUE,'el lede','las notas','Película',2026)",
    )
    .bind(gid)
    .execute(&pool)
    .await
    .unwrap();
    let pid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO photos (id, gallery_id, alt, thumbhash, width, height, position) \
         VALUES ($1,$2,'bride','abcd',4000,6000,0)",
    )
    .bind(pid)
    .bind(gid)
    .execute(&pool)
    .await
    .unwrap();
    for (fmt, w) in [("avif", 480), ("jpeg", 480)] {
        sqlx::query(
            "INSERT INTO photo_variants (photo_id, format, width, path) \
             VALUES ($1,$2,$3,$4)",
        )
        .bind(pid)
        .bind(fmt)
        .bind(w)
        .bind(format!("/media/{fmt}/x.{fmt}"))
        .execute(&pool)
        .await
        .unwrap();
    }

    let resp = app
        .oneshot(
            Request::get("/api/public/galleries/wd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let detail: GalleryDetail = body_json(resp).await;
    assert_eq!(detail.gallery.slug, "wd");
    assert_eq!(detail.gallery.description, "el lede");
    assert_eq!(detail.gallery.notes, "las notas");
    assert_eq!(detail.gallery.category, "Película");
    assert_eq!(detail.gallery.year, Some(2026));
    assert_eq!(detail.photos.len(), 1);
    assert_eq!(detail.photos[0].thumbhash, "abcd");
    assert_eq!(detail.photos[0].variants.len(), 2);
}

#[tokio::test]
#[serial_test::serial]
async fn public_posts_filter_and_detail() {
    let Some((app, pool, _media)) = setup().await else { return };
    sqlx::query(
        "INSERT INTO blog_posts (id, slug, title, body_blocks, status, published_at) \
         VALUES ($1,'p1','One',$2,'published',100)",
    )
    .bind(Uuid::new_v4())
    .bind(r#"[{"type":"heading","id":"h","level":2,"content":[{"text":"One"}]}]"#)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO blog_posts (id, slug, title, body_blocks, status, published_at) \
         VALUES ($1,'p2','Two','[]','draft',NULL)",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let list = app
        .clone()
        .oneshot(Request::get("/api/public/posts").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let posts: Vec<BlogPost> = body_json(list).await;
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].slug, "p1");

    let one = app
        .clone()
        .oneshot(
            Request::get("/api/public/posts/p1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(one.status(), StatusCode::OK);
    let detail: BlogPost = body_json(one).await;
    // The public site injects server-rendered HTML from the block document.
    assert_eq!(detail.body_html, "<h2>One</h2>");
    assert_eq!(detail.blocks.len(), 1);

    let draft = app
        .oneshot(
            Request::get("/api/public/posts/p2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(draft.status(), StatusCode::NOT_FOUND);
}
