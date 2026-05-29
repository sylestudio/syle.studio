//! API integration tests: admin create/upload/list endpoints.

mod common;
use common::*;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use syle_types::{BlogPost, Gallery, ImageFormat, Photo, PostStatus, UploadedImage};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[serial_test::serial]
async fn admin_create_gallery_requires_auth_and_persists() {
    let Some((app, pool, _media)) = setup().await else { return };

    let unauth = app
        .clone()
        .oneshot(
            Request::post("/api/admin/galleries")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"slug":"new","title":"New"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let cookie = login_cookie(&app, &pool).await;
    let created = app
        .clone()
        .oneshot(
            Request::post("/api/admin/galleries")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{"slug":"new","title":"New","published":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let g: Gallery = body_json(created).await;
    assert_eq!(g.slug, "new");

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM galleries WHERE slug='new'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
#[serial_test::serial]
async fn admin_create_post_requires_auth_and_persists() {
    let Some((app, pool, _media)) = setup().await else { return };

    let unauth = app
        .clone()
        .oneshot(
            Request::post("/api/admin/posts")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"slug":"x","title":"X","body_md":"b"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let cookie = login_cookie(&app, &pool).await;
    let created = app
        .oneshot(
            Request::post("/api/admin/posts")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r##"{"slug":"hello","title":"Hello","status":"published","blocks":[{"type":"heading","id":"h","level":1,"content":[{"text":"Hi"}]},{"type":"paragraph","id":"p","content":[{"text":"body"}]}]}"##,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let post: BlogPost = body_json(created).await;
    assert_eq!(post.slug, "hello");
    assert_eq!(post.status, PostStatus::Published);
    assert!(post.published_at.is_some());
    assert_eq!(post.blocks.len(), 2);
    // Create echoes back the same server-rendered HTML the public site will use.
    assert_eq!(post.body_html, "<h1>Hi</h1><p>body</p>");

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM blog_posts WHERE slug='hello'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
#[serial_test::serial]
async fn admin_list_galleries_includes_unpublished() {
    let Some((app, pool, _media)) = setup().await else { return };
    for (slug, pubd) in [("p", true), ("d", false)] {
        sqlx::query(
            "INSERT INTO galleries (id, slug, title, position, published) \
             VALUES ($1,$2,$3,0,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(slug)
        .bind(slug)
        .bind(pubd)
        .execute(&pool)
        .await
        .unwrap();
    }

    let unauth = app
        .clone()
        .oneshot(
            Request::get("/api/admin/galleries")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let cookie = login_cookie(&app, &pool).await;
    let resp = app
        .oneshot(
            Request::get("/api/admin/galleries")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let galleries: Vec<Gallery> = body_json(resp).await;
    assert_eq!(galleries.len(), 2);
}

#[tokio::test]
#[serial_test::serial]
async fn admin_list_posts_includes_drafts() {
    let Some((app, pool, _media)) = setup().await else { return };
    sqlx::query(
        "INSERT INTO blog_posts (id, slug, title, body_md, status, published_at) \
         VALUES ($1,'a','A','x','published',1),($2,'b','B','y','draft',NULL)",
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let cookie = login_cookie(&app, &pool).await;
    let resp = app
        .oneshot(
            Request::get("/api/admin/posts")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let posts: Vec<BlogPost> = body_json(resp).await;
    assert_eq!(posts.len(), 2);
}

#[tokio::test]
#[serial_test::serial]
async fn admin_upload_photo_ingests_and_persists() {
    let Some((app, pool, media)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;

    let gid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO galleries (id, slug, title, position, published) \
         VALUES ($1,'g','G',0,TRUE)",
    )
    .bind(gid)
    .execute(&pool)
    .await
    .unwrap();

    let (ct, body) = multipart(gid, "a sunset", &synthetic_png(1000, 700));
    let resp = app
        .oneshot(
            Request::post("/api/admin/photos")
                .header(header::CONTENT_TYPE, ct)
                .header(header::COOKIE, &cookie)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let photo: Photo = body_json(resp).await;

    assert_eq!(photo.gallery_id, gid);
    assert_eq!(photo.alt, "a sunset");
    assert_eq!((photo.width, photo.height), (1000, 700));
    assert!(!photo.thumbhash.is_empty());
    assert!(!photo.variants.is_empty());

    // Every advertised variant must exist on disk under media_dir.
    for v in &photo.variants {
        let rel = v.path.trim_start_matches('/');
        assert!(
            media.path().join(rel).exists(),
            "missing derivative file: {}",
            v.path
        );
    }

    let (pc, vc): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM photos WHERE gallery_id=$1), \
                (SELECT count(*) FROM photo_variants)",
    )
    .bind(gid)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pc, 1);
    assert_eq!(vc as usize, photo.variants.len());
}

#[tokio::test]
#[serial_test::serial]
async fn admin_upload_accepts_image_over_default_body_limit() {
    let Some((app, pool, _media)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;
    let gid = make_gallery(&pool, "big", true).await;

    // Real shoots run 30–50MB; this fixture only has to clear Axum's 2MB
    // default to prove the limit is lifted for the upload route.
    let big = noise_png(1400, 1100);
    assert!(
        big.len() > 2 * 1024 * 1024,
        "fixture must exceed the default 2MB limit, got {} bytes",
        big.len()
    );
    let (ct, body) = multipart(gid, "huge", &big);
    let resp = app
        .oneshot(
            Request::post("/api/admin/photos")
                .header(header::CONTENT_TYPE, ct)
                .header(header::COOKIE, &cookie)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
#[serial_test::serial]
async fn uploaded_derivative_is_served_under_media() {
    let Some((app, pool, _m)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;
    let gid = make_gallery(&pool, "g", true).await;

    let (ct, body) = multipart(gid, "x", &synthetic_png(640, 480));
    let up = app
        .clone()
        .oneshot(
            Request::post("/api/admin/photos")
                .header(header::CONTENT_TYPE, ct)
                .header(header::COOKIE, &cookie)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let photo: Photo = body_json(up).await;

    let served = app
        .oneshot(
            Request::get(&photo.variants[0].path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(served.status(), StatusCode::OK);
}

#[tokio::test]
#[serial_test::serial]
async fn admin_blog_asset_upload_ingests_and_is_served_without_a_gallery() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;

    // Wide enough that several responsive widths survive (480/960/1440).
    let (ct, body) = multipart_file(&synthetic_png(1600, 1067));
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/admin/blog-assets")
                .header(header::CONTENT_TYPE, ct)
                .header(header::COOKIE, &cookie)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let asset: UploadedImage = body_json(resp).await;

    // Same optimization as galleries: responsive AVIF+JPEG renditions across
    // several widths, a blur-up placeholder, and source dimensions.
    assert_eq!((asset.width, asset.height), (1600, 1067));
    assert!(asset.src.starts_with("/media/jpeg/"));
    assert!(asset
        .variants
        .iter()
        .any(|v| v.format == ImageFormat::Avif));
    let widths: std::collections::BTreeSet<u32> =
        asset.variants.iter().map(|v| v.width).collect();
    assert!(widths.len() >= 2, "expected multiple responsive widths");
    assert!(asset.placeholder.starts_with("data:image/png;base64,"));

    // Both an AVIF rendition and the JPEG fallback are actually served.
    let avif = asset
        .variants
        .iter()
        .find(|v| v.format == ImageFormat::Avif)
        .unwrap();
    for path in [&asset.src, &avif.path] {
        let served = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(served.status(), StatusCode::OK);
    }

    // A blog image must not create gallery/photo rows.
    let (photos,): (i64,) = sqlx::query_as("SELECT count(*) FROM photos")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(photos, 0);
}

#[tokio::test]
#[serial_test::serial]
async fn admin_blog_asset_upload_requires_auth() {
    let Some((app, _pool, _media)) = setup().await else {
        return;
    };
    let (ct, body) = multipart_file(&synthetic_png(64, 64));
    let resp = app
        .oneshot(
            Request::post("/api/admin/blog-assets")
                .header(header::CONTENT_TYPE, ct)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
