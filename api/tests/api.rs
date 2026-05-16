//! API integration tests: public read filtering + Argon2id/session auth.
//! Runs against the dev DB (workspace `.env`); skips cleanly if unset.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use syle_api::{app, auth::hash_password, AppState};
use syle_types::{Gallery, User};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup() -> Option<(axum::Router, sqlx::PgPool)> {
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
    let state = AppState::new(pool.clone(), "/tmp/syle-media-test".into());
    Some((app(state), pool))
}

async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn public_galleries_lists_only_published() {
    let Some((app, pool)) = setup().await else { return };
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
async fn login_issues_session_cookie_and_guards_me() {
    let Some((app, pool)) = setup().await else { return };
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
