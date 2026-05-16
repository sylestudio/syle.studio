//! API integration tests: slug well-formedness + post slug persistence.

mod common;
use common::*;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use syle_types::BlogPost;
use tower::ServiceExt;

async fn new_post(app: &axum::Router, cookie: &str, slug: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::post("/api/admin/posts")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(format!(
                    r#"{{"slug":"{slug}","title":"T","body_md":"b","status":"draft"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
#[serial_test::serial]
async fn update_post_persists_slug() {
    let Some((app, pool, _m)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;

    let created: BlogPost = body_json(new_post(&app, &cookie, "old-slug").await).await;

    let resp = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/admin/posts/{}", created.id))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"slug":"new-slug"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let updated: BlogPost = body_json(resp).await;
    assert_eq!(updated.slug, "new-slug");
}

#[tokio::test]
#[serial_test::serial]
async fn create_post_rejects_invalid_slug() {
    let Some((app, pool, _m)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;

    let resp = new_post(&app, &cookie, "j.pn").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial_test::serial]
async fn update_post_rejects_invalid_slug() {
    let Some((app, pool, _m)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;
    let created: BlogPost = body_json(new_post(&app, &cookie, "good-slug").await).await;

    let resp = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/admin/posts/{}", created.id))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"slug":"j.pn"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial_test::serial]
async fn create_gallery_rejects_invalid_slug() {
    let Some((app, pool, _m)) = setup().await else { return };
    let cookie = login_cookie(&app, &pool).await;

    let resp = app
        .oneshot(
            Request::post("/api/admin/galleries")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"slug":"Bad Slug","title":"T"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial_test::serial]
async fn update_gallery_rejects_invalid_slug() {
    let Some((app, pool, _m)) = setup().await else { return };
    let gid = make_gallery(&pool, "ok", false).await;
    let cookie = login_cookie(&app, &pool).await;

    let resp = app
        .oneshot(
            Request::patch(format!("/api/admin/galleries/{gid}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"slug":"a--b"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
