//! API integration tests: Argon2id/session auth.

mod common;
use common::*;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use syle_api::auth::hash_password;
use syle_types::User;
use tower::ServiceExt;
use uuid::Uuid;

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
