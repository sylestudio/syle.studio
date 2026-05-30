//! Access-log audit trail: authentication events (password + recovery logins,
//! logout, recovery-code generation) are recorded with best-effort request
//! metadata and surfaced read-only behind the session guard.

mod common;
use common::*;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use syle_api::auth::hash_password;
use syle_types::{AccessAction, AccessLogEntry, AccessMethod, AccessOutcome, RecoveryCodes};
use tower::ServiceExt;
use uuid::Uuid;

async fn seed_user(pool: &sqlx::PgPool, email: &str, pw: &str) {
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1,$2,$3)")
        .bind(Uuid::new_v4())
        .bind(email)
        .bind(hash_password(pw).unwrap())
        .execute(pool)
        .await
        .unwrap();
}

fn cookie_of(resp: &axum::response::Response) -> String {
    resp.headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn login(
    app: &axum::Router,
    email: &str,
    pw: &str,
    ua: &str,
    ip: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::post("/api/admin/login")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::USER_AGENT, ua)
                .header("x-forwarded-for", ip)
                .body(Body::from(format!(
                    r#"{{"email":"{email}","password":"{pw}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get_log(app: &axum::Router, cookie: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::get("/api/admin/access-log")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
#[serial_test::serial]
async fn logins_recorded_with_metadata_recent_first() {
    let Some((app, pool, _m)) = setup().await else {
        return;
    };
    seed_user(&pool, "op@syle.studio", "pw").await;

    let bad = login(&app, "op@syle.studio", "nope", "UA-bad", "9.9.9.9").await;
    assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
    let ghost = login(&app, "ghost@syle.studio", "x", "UA-ghost", "8.8.8.8").await;
    assert_eq!(ghost.status(), StatusCode::UNAUTHORIZED);
    let ok = login(&app, "op@syle.studio", "pw", "UA-good", "1.2.3.4").await;
    assert_eq!(ok.status(), StatusCode::OK);
    let cookie = cookie_of(&ok);

    let log: Vec<AccessLogEntry> = body_json(get_log(&app, &cookie).await).await;

    // Most-recent-first: the successful login is the latest event, fully tagged.
    let top = &log[0];
    assert_eq!(top.action, AccessAction::Login);
    assert_eq!(top.method, Some(AccessMethod::Password));
    assert_eq!(top.outcome, AccessOutcome::Success);
    assert_eq!(top.email.as_deref(), Some("op@syle.studio"));
    assert_eq!(top.ip.as_deref(), Some("1.2.3.4"));
    assert_eq!(top.user_agent.as_deref(), Some("UA-good"));

    // An unknown email still records a failure (no user resolved) with the
    // attempted email — exactly the recon signal worth seeing.
    let g = log
        .iter()
        .find(|e| e.email.as_deref() == Some("ghost@syle.studio"))
        .expect("unknown-email attempt logged");
    assert_eq!(g.outcome, AccessOutcome::Failure);
    assert_eq!(g.method, Some(AccessMethod::Password));

    // Wrong password against a known account is a failure too.
    assert!(log.iter().any(|e| e.outcome == AccessOutcome::Failure
        && e.email.as_deref() == Some("op@syle.studio")
        && e.user_agent.as_deref() == Some("UA-bad")));
}

#[tokio::test]
#[serial_test::serial]
async fn access_log_requires_session() {
    let Some((app, _pool, _m)) = setup().await else {
        return;
    };
    let resp = app
        .oneshot(
            Request::get("/api/admin/access-log")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial_test::serial]
async fn logout_is_recorded() {
    let Some((app, pool, _m)) = setup().await else {
        return;
    };
    seed_user(&pool, "op@syle.studio", "pw").await;

    let cookie = cookie_of(&login(&app, "op@syle.studio", "pw", "UA", "1.1.1.1").await);
    let out = app
        .clone()
        .oneshot(
            Request::post("/api/admin/logout")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.status(), StatusCode::OK);

    // Re-authenticate to read the trail (the prior session is now revoked).
    let cookie2 = cookie_of(&login(&app, "op@syle.studio", "pw", "UA", "1.1.1.1").await);
    let log: Vec<AccessLogEntry> = body_json(get_log(&app, &cookie2).await).await;
    assert!(log.iter().any(|e| e.action == AccessAction::Logout));
}

#[tokio::test]
#[serial_test::serial]
async fn recovery_events_are_recorded() {
    let Some((app, pool, _m)) = setup().await else {
        return;
    };
    seed_user(&pool, "op@syle.studio", "pw").await;
    let cookie = cookie_of(&login(&app, "op@syle.studio", "pw", "UA", "1.1.1.1").await);

    // Generating codes is itself an audited security event.
    let gen = app
        .clone()
        .oneshot(
            Request::post("/api/admin/recovery/generate")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(gen.status(), StatusCode::OK);
    let codes: RecoveryCodes = body_json(gen).await;

    // A well-formed but wrong code → recorded failure (account resolved).
    let miss = app
        .clone()
        .oneshot(
            Request::post("/api/admin/recovery/redeem")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"email":"op@syle.studio","code":"AAAAA-AAAAA"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(miss.status(), StatusCode::UNAUTHORIZED);

    // A real code → success, mints a session we then use to read the log.
    let hit = app
        .clone()
        .oneshot(
            Request::post("/api/admin/recovery/redeem")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"op@syle.studio","code":"{}"}}"#,
                    codes.codes[0]
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hit.status(), StatusCode::OK);
    let cookie2 = cookie_of(&hit);

    let log: Vec<AccessLogEntry> = body_json(get_log(&app, &cookie2).await).await;
    assert!(log
        .iter()
        .any(|e| e.action == AccessAction::Login
            && e.method == Some(AccessMethod::Recovery)
            && e.outcome == AccessOutcome::Success));
    assert!(log
        .iter()
        .any(|e| e.action == AccessAction::Login
            && e.method == Some(AccessMethod::Recovery)
            && e.outcome == AccessOutcome::Failure));
    assert!(log
        .iter()
        .any(|e| e.action == AccessAction::RecoveryGenerate));
}
