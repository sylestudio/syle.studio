//! API integration tests: curation (admin detail/patch/delete).

mod common;
use common::*;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use syle_types::{BlogPost, Gallery, GalleryDetail, Photo, PostStatus};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[serial_test::serial]
async fn admin_gallery_detail_includes_unpublished() {
    let Some((app, pool, _m)) = setup().await else { return };
    let gid = make_gallery(&pool, "draft-g", false).await;
    make_photo(&pool, gid, 0).await;
    let cookie = login_cookie(&app, &pool).await;

    let unauth = app
        .clone()
        .oneshot(
            Request::get(format!("/api/admin/galleries/{gid}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let resp = app
        .oneshot(
            Request::get(format!("/api/admin/galleries/{gid}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let d: GalleryDetail = body_json(resp).await;
    assert_eq!(d.gallery.slug, "draft-g");
    assert_eq!(d.photos.len(), 1);
}

#[tokio::test]
#[serial_test::serial]
async fn patch_gallery_publishes() {
    let Some((app, pool, _m)) = setup().await else { return };
    let gid = make_gallery(&pool, "g", false).await;
    let cookie = login_cookie(&app, &pool).await;

    let resp = app
        .oneshot(
            Request::patch(format!("/api/admin/galleries/{gid}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"published":true,"title":"New T"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let g: Gallery = body_json(resp).await;
    assert!(g.published);
    assert_eq!(g.title, "New T");
}

#[tokio::test]
#[serial_test::serial]
async fn patch_gallery_sets_editorial_fields_and_partial_patch_preserves_year() {
    let Some((app, pool, _m)) = setup().await else { return };
    let gid = make_gallery(&pool, "g", false).await;
    let cookie = login_cookie(&app, &pool).await;

    let resp = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/admin/galleries/{gid}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{"description":"el lede","notes":"las notas",
                        "category":"Dirección · Prenda","year":2026}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let g: Gallery = body_json(resp).await;
    assert_eq!(g.description, "el lede");
    assert_eq!(g.notes, "las notas");
    assert_eq!(g.category, "Dirección · Prenda");
    assert_eq!(g.year, Some(2026));

    let (desc, year): (String, Option<i32>) =
        sqlx::query_as("SELECT description, year FROM galleries WHERE id=$1")
            .bind(gid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(desc, "el lede");
    assert_eq!(year, Some(2026));

    // A partial patch that omits `year` must not wipe it (COALESCE merge).
    let r2 = app
        .oneshot(
            Request::patch(format!("/api/admin/galleries/{gid}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"title":"Otro título"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::OK);
    let g2: Gallery = body_json(r2).await;
    assert_eq!(g2.title, "Otro título");
    assert_eq!(g2.year, Some(2026));
    assert_eq!(g2.description, "el lede");
}

#[tokio::test]
#[serial_test::serial]
async fn patch_photo_updates_alt_and_reorder_sets_positions() {
    let Some((app, pool, _m)) = setup().await else { return };
    let gid = make_gallery(&pool, "g", true).await;
    let p0 = make_photo(&pool, gid, 0).await;
    let p1 = make_photo(&pool, gid, 1).await;
    let p2 = make_photo(&pool, gid, 2).await;
    let cookie = login_cookie(&app, &pool).await;

    let resp = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/admin/photos/{p0}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"alt":"sunset"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let alt: (String,) = sqlx::query_as("SELECT alt FROM photos WHERE id=$1")
        .bind(p0)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(alt.0, "sunset");

    // Reverse order: p2,p1,p0 -> positions 0,1,2
    let body = format!(r#"{{"ids":["{p2}","{p1}","{p0}"]}}"#);
    let r = app
        .oneshot(
            Request::patch(format!("/api/admin/galleries/{gid}/photos/order"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let pos: (i32,) = sqlx::query_as("SELECT position FROM photos WHERE id=$1")
        .bind(p2)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(pos.0, 0);
}

#[tokio::test]
#[serial_test::serial]
async fn delete_photo_and_gallery_remove_files_and_rows() {
    let Some((app, pool, media)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;
    let gid = make_gallery(&pool, "g", true).await;

    let (ct, body) = multipart(gid, "x", &synthetic_png(800, 600));
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
    let file = media
        .path()
        .join(photo.variants[0].path.trim_start_matches('/'));
    assert!(file.exists());

    let pid = photo.id;
    let d = app
        .clone()
        .oneshot(
            Request::delete(format!("/api/admin/photos/{pid}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(d.status(), StatusCode::OK);
    assert!(!file.exists(), "variant file should be deleted");
    let (cnt,): (i64,) = sqlx::query_as("SELECT count(*) FROM photos WHERE id=$1")
        .bind(pid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cnt, 0);

    let dg = app
        .oneshot(
            Request::delete(format!("/api/admin/galleries/{gid}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dg.status(), StatusCode::OK);
    let (gc,): (i64,) = sqlx::query_as("SELECT count(*) FROM galleries WHERE id=$1")
        .bind(gid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(gc, 0);
}

#[tokio::test]
#[serial_test::serial]
async fn patch_post_publish_and_delete() {
    let Some((app, pool, _m)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;
    let pid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO blog_posts (id, slug, title, body_md, status, published_at) \
         VALUES ($1,'s','T','b','draft',NULL)",
    )
    .bind(pid)
    .execute(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/admin/posts/{pid}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"status":"published"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let post: BlogPost = body_json(r).await;
    assert_eq!(post.status, PostStatus::Published);
    assert!(post.published_at.is_some());

    let d = app
        .oneshot(
            Request::delete(format!("/api/admin/posts/{pid}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(d.status(), StatusCode::OK);
    let (c,): (i64,) = sqlx::query_as("SELECT count(*) FROM blog_posts WHERE id=$1")
        .bind(pid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(c, 0);
}

#[tokio::test]
#[serial_test::serial]
async fn admin_get_post_returns_draft_and_404s() {
    let Some((app, pool, _m)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;
    let pid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO blog_posts (id, slug, title, body_md, status, published_at) \
         VALUES ($1,'d','Draft','# x','draft',NULL)",
    )
    .bind(pid)
    .execute(&pool)
    .await
    .unwrap();

    let unauth = app
        .clone()
        .oneshot(
            Request::get(format!("/api/admin/posts/{pid}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let ok = app
        .clone()
        .oneshot(
            Request::get(format!("/api/admin/posts/{pid}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    let p: BlogPost = body_json(ok).await;
    assert_eq!(p.status, PostStatus::Draft);

    let missing = app
        .oneshot(
            Request::get(format!("/api/admin/posts/{}", Uuid::new_v4()))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}
