#![allow(dead_code)]
//! Shared helpers for API integration tests.

use axum::body::Body;
use axum::http::{header, Request};
use http_body_util::BodyExt;
use image::{ImageFormat as ImgFmt, RgbImage};
use std::io::Cursor;
use syle_api::{app, auth::build_webauthn, auth::hash_password, AppState};
use tower::ServiceExt;
use uuid::Uuid;

pub async fn setup() -> Option<(axum::Router, sqlx::PgPool, tempfile::TempDir)> {
    dotenvy::from_path("../.env").ok();
    // Tests TRUNCATE; never point them at the dev database. Prefer the
    // dedicated TEST_DATABASE_URL and fall back to DATABASE_URL only if unset.
    let url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()?;
    let pool = syle_core::db::connect(&url).await.unwrap();
    syle_core::db::migrate(&pool).await.unwrap();
    sqlx::query(
        "TRUNCATE projects, galleries, photos, photo_variants, blog_posts, users, sessions, \
         webauthn_credentials, webauthn_flows, recovery_codes, access_log CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();
    let media = tempfile::tempdir().unwrap();
    let webauthn = build_webauthn("localhost", "http://localhost:8080").unwrap();
    let state = AppState::new(pool.clone(), media.path().to_path_buf(), webauthn);
    Some((app(state), pool, media))
}

pub async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

pub async fn login_cookie(app: &axum::Router, pool: &sqlx::PgPool) -> String {
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

pub fn synthetic_png(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImgFmt::Png).unwrap();
    buf
}

/// Incompressible PNG: pseudo-random pixels (LCG) so deflate can't shrink it,
/// yielding an encoded size ≈ raw RGB. Used to exceed Axum's default 2MB body
/// limit with a payload `ingest` can still decode.
pub fn noise_png(w: u32, h: u32) -> Vec<u8> {
    let mut s: u32 = 0x9E37_79B9;
    let img = RgbImage::from_fn(w, h, |_, _| {
        let mut c = [0u8; 3];
        for b in &mut c {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = (s >> 24) as u8;
        }
        image::Rgb(c)
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImgFmt::Png).unwrap();
    buf
}

/// Build a minimal multipart/form-data body: gallery_id, alt, file.
pub fn multipart(gallery_id: Uuid, alt: &str, png: &[u8]) -> (String, Vec<u8>) {
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

/// Multipart body with only a `file` field (inline blog-image upload).
pub fn multipart_file(png: &[u8]) -> (String, Vec<u8>) {
    let b = "BOUND";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"p.png\"\r\nContent-Type: image/png\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(png);
    body.extend_from_slice(format!("\r\n--{b}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={b}"), body)
}

pub async fn make_gallery(pool: &sqlx::PgPool, slug: &str, published: bool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO galleries (id, slug, title, position, published) \
         VALUES ($1,$2,$3,0,$4)",
    )
    .bind(id)
    .bind(slug)
    .bind(slug)
    .bind(published)
    .execute(pool)
    .await
    .unwrap();
    id
}

pub async fn make_photo(pool: &sqlx::PgPool, gid: Uuid, pos: i32) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO photos (id, gallery_id, alt, thumbhash, width, height, position) \
         VALUES ($1,$2,'a','hh',10,10,$3)",
    )
    .bind(id)
    .bind(gid)
    .bind(pos)
    .execute(pool)
    .await
    .unwrap();
    id
}
