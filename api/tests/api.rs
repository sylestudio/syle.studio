//! API integration tests: public read filtering + Argon2id/session auth.
//! Runs against the dev DB (workspace `.env`); skips cleanly if unset.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use image::{ImageFormat as ImgFmt, RgbImage};
use std::io::Cursor;
use syle_api::{app, auth::hash_password, AppState};
use syle_types::{BlogPost, Gallery, GalleryDetail, Photo, PostStatus, User};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup() -> Option<(axum::Router, sqlx::PgPool, tempfile::TempDir)> {
    dotenvy::from_path("../.env").ok();
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = syle_core::db::connect(&url).await.unwrap();
    syle_core::db::migrate(&pool).await.unwrap();
    sqlx::query(
        "TRUNCATE galleries, photos, photo_variants, blog_posts, users, sessions CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();
    let media = tempfile::tempdir().unwrap();
    let state = AppState::new(pool.clone(), media.path().to_path_buf());
    Some((app(state), pool, media))
}

async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

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
async fn login_issues_session_cookie_and_guards_me() {
    let Some((app, pool, _media)) = setup().await else { return };
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1,$2,$3)")
        .bind(Uuid::new_v4())
        .bind("op@syle.studio")
        .bind(hash_password("correct horse").unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let bad = app
        .clone()
        .oneshot(
            Request::post("/api/admin/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"email":"op@syle.studio","password":"wrong"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);

    let ok = app
        .clone()
        .oneshot(
            Request::post("/api/admin/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"email":"op@syle.studio","password":"correct horse"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    let cookie = ok
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(cookie.contains("sid="));
    assert!(cookie.contains("HttpOnly"));

    let no_cookie = app
        .clone()
        .oneshot(Request::get("/api/admin/me").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(no_cookie.status(), StatusCode::UNAUTHORIZED);

    let me = app
        .oneshot(
            Request::get("/api/admin/me")
                .header(header::COOKIE, cookie.split(';').next().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    let user: User = body_json(me).await;
    assert_eq!(user.email, "op@syle.studio");
}

async fn login_cookie(app: &axum::Router, pool: &sqlx::PgPool) -> String {
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1,$2,$3)")
        .bind(Uuid::new_v4())
        .bind("op@syle.studio")
        .bind(hash_password("pw").unwrap())
        .execute(pool)
        .await
        .unwrap();
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/admin/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"email":"op@syle.studio","password":"pw"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let c = resp
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    c.split(';').next().unwrap().to_string()
}

#[tokio::test]
#[serial_test::serial]
async fn gallery_detail_returns_photos_with_variants() {
    let Some((app, pool, _media)) = setup().await else { return };
    let gid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO galleries (id, slug, title, position, published) \
         VALUES ($1,'wd','Wedding',0,TRUE)",
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
    assert_eq!(detail.photos.len(), 1);
    assert_eq!(detail.photos[0].thumbhash, "abcd");
    assert_eq!(detail.photos[0].variants.len(), 2);
}

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
async fn public_posts_filter_and_detail() {
    let Some((app, pool, _media)) = setup().await else { return };
    sqlx::query(
        "INSERT INTO blog_posts (id, slug, title, body_md, status, published_at) \
         VALUES ($1,'p1','One','# one','published',100)",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO blog_posts (id, slug, title, body_md, status, published_at) \
         VALUES ($1,'p2','Two','# two','draft',NULL)",
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
                    r##"{"slug":"hello","title":"Hello","body_md":"# hi","status":"published"}"##,
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

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM blog_posts WHERE slug='hello'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

fn synthetic_png(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImgFmt::Png).unwrap();
    buf
}

/// Build a minimal multipart/form-data body: gallery_id, alt, file.
fn multipart(gallery_id: Uuid, alt: &str, png: &[u8]) -> (String, Vec<u8>) {
    let b = "BOUND";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!("--{b}\r\nContent-Disposition: form-data; name=\"gallery_id\"\r\n\r\n{gallery_id}\r\n").as_bytes(),
    );
    body.extend_from_slice(
        format!("--{b}\r\nContent-Disposition: form-data; name=\"alt\"\r\n\r\n{alt}\r\n").as_bytes(),
    );
    body.extend_from_slice(
        format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"p.png\"\r\nContent-Type: image/png\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(png);
    body.extend_from_slice(format!("\r\n--{b}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={b}"), body)
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
