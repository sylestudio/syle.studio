//! WebAuthn ceremony integration tests, driven browserless via `SoftPasskey`
//! over the real HTTP router. Mirrors the Gate #0 round-trip, but across the
//! wire and through the DB-backed flow store.

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::{body_json, login_cookie, setup};
use serde_json::{json, Value};
use syle_types::{CredentialInfo, FlowChallenge, RecoveryCodes};
use tower::ServiceExt;
use url::Url;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_rs::prelude::{
    CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse,
};

const ORIGIN: &str = "http://localhost:8080";

/// POST JSON, optionally carrying a session cookie. `body` of `Value::Null`
/// sends an empty body (for the cookie-only `register/start`).
async fn post(
    app: &axum::Router,
    uri: &str,
    cookie: Option<&str>,
    body: &Value,
) -> axum::response::Response {
    let mut req = Request::post(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let payload = if body.is_null() {
        Body::empty()
    } else {
        Body::from(serde_json::to_vec(body).unwrap())
    };
    app.clone().oneshot(req.body(payload).unwrap()).await.unwrap()
}

/// Drive a passkey registration end to end against the live router, returning
/// the registered `CredentialInfo`. Reuses `auth` so its key store persists for
/// any follow-up authentication in the same test.
async fn register(
    app: &axum::Router,
    cookie: &str,
    auth: &mut WebauthnAuthenticator<SoftPasskey>,
) -> CredentialInfo {
    let resp = post(app, "/api/admin/webauthn/register/start", Some(cookie), &Value::Null).await;
    assert_eq!(resp.status(), StatusCode::OK, "register/start");
    let challenge: FlowChallenge = body_json(resp).await;

    let ccr: CreationChallengeResponse =
        serde_json::from_value(challenge.options).expect("options -> CreationChallengeResponse");
    let rpkc: RegisterPublicKeyCredential = auth
        .do_registration(Url::parse(ORIGIN).unwrap(), ccr)
        .expect("authenticator registration");

    let finish = json!({
        "flow_id": challenge.flow_id,
        "credential": serde_json::to_value(&rpkc).unwrap(),
    });
    let resp = post(app, "/api/admin/webauthn/register/finish", Some(cookie), &finish).await;
    assert_eq!(resp.status(), StatusCode::OK, "register/finish");
    body_json(resp).await
}

/// Drive an email-first passkey login end to end (pre-auth), returning the
/// `login/finish` response so the caller can inspect the minted cookie.
async fn login(
    app: &axum::Router,
    auth: &mut WebauthnAuthenticator<SoftPasskey>,
) -> axum::response::Response {
    let resp = post(
        app,
        "/api/admin/webauthn/login/start",
        None,
        &json!({ "email": "op@syle.studio" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "login/start");
    let challenge: FlowChallenge = body_json(resp).await;

    let rcr: RequestChallengeResponse =
        serde_json::from_value(challenge.options).expect("options -> RequestChallengeResponse");
    let pkc: PublicKeyCredential = auth
        .do_authentication(Url::parse(ORIGIN).unwrap(), rcr)
        .expect("authenticator authentication");

    let finish = json!({
        "flow_id": challenge.flow_id,
        "credential": serde_json::to_value(&pkc).unwrap(),
    });
    post(app, "/api/admin/webauthn/login/finish", None, &finish).await
}

async fn get(app: &axum::Router, uri: &str, cookie: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::get(uri)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn delete(app: &axum::Router, uri: &str, cookie: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::delete(uri)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn patch(
    app: &axum::Router,
    uri: &str,
    cookie: &str,
    body: &Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::patch(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(serde_json::to_vec(body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
#[serial_test::serial]
async fn register_persists_credential_and_consumes_flow() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));

    let info = register(&app, &cookie, &mut authenticator).await;
    assert_eq!(info.last_used_at, None);

    // Exactly one credential persisted for the operator.
    let creds: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webauthn_credentials")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(creds, 1, "one credential row");

    // The ceremony flow was consumed (single-use).
    let flows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webauthn_flows")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(flows, 0, "register flow consumed");
}

#[tokio::test]
#[serial_test::serial]
async fn passkey_login_issues_reusable_session() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    register(&app, &cookie, &mut authenticator).await;

    let resp = login(&app, &mut authenticator).await;
    assert_eq!(resp.status(), StatusCode::OK, "login/finish");
    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .expect("sets a cookie")
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.contains("sid="), "issues sid cookie");
    assert!(set_cookie.contains("HttpOnly"), "cookie is HttpOnly");

    // The minted session works on a guarded route — proves `issue_session` reuse
    // (byte-identical to the password path the `AuthUser` extractor expects).
    let session = set_cookie.split(';').next().unwrap();
    let me = get(&app, "/api/admin/me", session).await;
    assert_eq!(me.status(), StatusCode::OK, "session authenticates /me");
}

#[tokio::test]
#[serial_test::serial]
async fn login_start_requires_a_registered_passkey() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    // The operator exists (login_cookie inserts them) but has no passkey yet.
    let _ = login_cookie(&app, &pool).await;
    let resp = post(
        &app,
        "/api/admin/webauthn/login/start",
        None,
        &json!({ "email": "op@syle.studio" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "no passkey enrolled");

    // An unknown email is rejected the same way.
    let resp = post(
        &app,
        "/api/admin/webauthn/login/start",
        None,
        &json!({ "email": "nobody@syle.studio" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "unknown email");
}

#[tokio::test]
#[serial_test::serial]
async fn login_persists_sign_counter_and_last_used() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    register(&app, &cookie, &mut authenticator).await;

    assert_eq!(login(&app, &mut authenticator).await.status(), StatusCode::OK);
    let (pk1, used1): (String, Option<i64>) =
        sqlx::query_as("SELECT passkey::text, last_used_at FROM webauthn_credentials")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(used1.is_some(), "last_used_at stamped after login");

    // The soft authenticator bumps its counter on each auth, so the persisted
    // passkey JSONB must change between logins — proof the sign-counter update
    // is stored (clone detection stays meaningful).
    assert_eq!(login(&app, &mut authenticator).await.status(), StatusCode::OK);
    let (pk2, _used2): (String, Option<i64>) =
        sqlx::query_as("SELECT passkey::text, last_used_at FROM webauthn_credentials")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(pk1, pk2, "sign counter advanced and was persisted");
}

#[tokio::test]
#[serial_test::serial]
async fn recovery_code_is_single_use() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;

    // Generate a fresh set (authenticated).
    let resp = post(&app, "/api/admin/recovery/generate", Some(&cookie), &Value::Null).await;
    assert_eq!(resp.status(), StatusCode::OK, "recovery/generate");
    let set: RecoveryCodes = body_json(resp).await;
    assert_eq!(set.codes.len(), 10, "ten codes issued");
    let code = set.codes[0].clone();

    // A malformed code is rejected by the format gate (no account takeover, and
    // cheap — no hashing).
    let resp = post(
        &app,
        "/api/admin/recovery/redeem",
        None,
        &json!({ "email": "op@syle.studio", "code": "nope" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "malformed code");

    // Redeem once (pre-auth) → mints a usable session.
    let resp = post(
        &app,
        "/api/admin/recovery/redeem",
        None,
        &json!({ "email": "op@syle.studio", "code": code }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "first redeem");
    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .expect("sets a cookie")
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.contains("sid="), "issues sid cookie");
    let session = set_cookie.split(';').next().unwrap();
    assert_eq!(
        get(&app, "/api/admin/me", session).await.status(),
        StatusCode::OK,
        "recovery session authenticates"
    );

    // The same code can never be redeemed again.
    let resp = post(
        &app,
        "/api/admin/recovery/redeem",
        None,
        &json!({ "email": "op@syle.studio", "code": code }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "second redeem rejected");
}

#[tokio::test]
#[serial_test::serial]
async fn credentials_list_then_delete_with_password_fallback() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let info = register(&app, &cookie, &mut authenticator).await;

    let listed: Vec<CredentialInfo> =
        body_json(get(&app, "/api/admin/webauthn/credentials", &cookie).await).await;
    assert_eq!(listed.len(), 1, "one credential listed");
    assert_eq!(listed[0].id, info.id);

    // The operator still has a password, so removing the only passkey is allowed.
    let resp = delete(
        &app,
        &format!("/api/admin/webauthn/credentials/{}", info.id),
        &cookie,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "delete allowed with password");

    let listed: Vec<CredentialInfo> =
        body_json(get(&app, "/api/admin/webauthn/credentials", &cookie).await).await;
    assert!(listed.is_empty(), "credential revoked");
}

#[tokio::test]
#[serial_test::serial]
async fn cannot_delete_the_last_authentication_factor() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let info = register(&app, &cookie, &mut authenticator).await;

    // Make the account passkey-only (no password) with no recovery codes: the
    // single passkey is now the last factor.
    sqlx::query("UPDATE users SET password_hash = NULL WHERE email = 'op@syle.studio'")
        .execute(&pool)
        .await
        .unwrap();

    let resp = delete(
        &app,
        &format!("/api/admin/webauthn/credentials/{}", info.id),
        &cookie,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "last factor protected");

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webauthn_credentials")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 1, "credential not removed");
}

#[tokio::test]
#[serial_test::serial]
async fn credential_can_be_renamed() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let info = register(&app, &cookie, &mut authenticator).await;
    assert_eq!(info.name, "", "starts unnamed");

    let resp = patch(
        &app,
        &format!("/api/admin/webauthn/credentials/{}", info.id),
        &cookie,
        &json!({ "name": "MacBook Touch ID" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "rename");
    let updated: CredentialInfo = body_json(resp).await;
    assert_eq!(updated.name, "MacBook Touch ID");

    let listed: Vec<CredentialInfo> =
        body_json(get(&app, "/api/admin/webauthn/credentials", &cookie).await).await;
    assert_eq!(listed[0].name, "MacBook Touch ID", "name persisted");
}

#[tokio::test]
#[serial_test::serial]
async fn passkey_actions_are_audited() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));

    let info = register(&app, &cookie, &mut authenticator).await; // enroll
    assert_eq!(login(&app, &mut authenticator).await.status(), StatusCode::OK); // passkey login
    let resp = delete(
        &app,
        &format!("/api/admin/webauthn/credentials/{}", info.id),
        &cookie,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK); // revoke

    let enroll: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM access_log WHERE action = 'passkey_enroll' AND outcome = 'success'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(enroll, 1, "enroll audited");
    let logged_in: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM access_log \
         WHERE action = 'login' AND method = 'passkey' AND outcome = 'success'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(logged_in, 1, "passkey login audited");
    let revoke: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM access_log WHERE action = 'passkey_revoke'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(revoke, 1, "revoke audited");
}

#[tokio::test]
#[serial_test::serial]
async fn failed_passkey_login_is_audited() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    register(&app, &cookie, &mut authenticator).await;

    // A valid flow, then a bogus credential at finish → failed access attempt.
    let resp = post(
        &app,
        "/api/admin/webauthn/login/start",
        None,
        &json!({ "email": "op@syle.studio" }),
    )
    .await;
    let challenge: FlowChallenge = body_json(resp).await;
    let finish = json!({ "flow_id": challenge.flow_id, "credential": { "bogus": true } });
    let resp = post(&app, "/api/admin/webauthn/login/finish", None, &finish).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "bogus credential rejected");

    let failed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM access_log \
         WHERE action = 'login' AND method = 'passkey' AND outcome = 'failure'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(failed, 1, "failed passkey login audited");
}

#[tokio::test]
#[serial_test::serial]
async fn expired_register_flow_is_rejected() {
    let Some((app, pool, _media)) = setup().await else {
        return;
    };
    let cookie = login_cookie(&app, &pool).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));

    let resp = post(&app, "/api/admin/webauthn/register/start", Some(&cookie), &Value::Null).await;
    assert_eq!(resp.status(), StatusCode::OK, "register/start");
    let challenge: FlowChallenge = body_json(resp).await;

    // Force the parked flow to be expired before it is finished.
    sqlx::query("UPDATE webauthn_flows SET expires_at = 0 WHERE flow_id = $1")
        .bind(&challenge.flow_id)
        .execute(&pool)
        .await
        .unwrap();

    let ccr: CreationChallengeResponse = serde_json::from_value(challenge.options).unwrap();
    let rpkc: RegisterPublicKeyCredential = authenticator
        .do_registration(Url::parse(ORIGIN).unwrap(), ccr)
        .unwrap();
    let finish = json!({
        "flow_id": challenge.flow_id,
        "credential": serde_json::to_value(&rpkc).unwrap(),
    });
    let resp = post(&app, "/api/admin/webauthn/register/finish", Some(&cookie), &finish).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "expired flow rejected");

    let creds: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webauthn_credentials")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(creds, 0, "no credential persisted from an expired flow");
}
