//! Access-log audit trail. A small infallible recorder is called from every
//! authentication path; `list_access_log` serves the trail read-only to the CRM.
//!
//! Everything but `action`/`outcome` may be attacker-controlled (a pre-auth
//! login carries whatever email / User-Agent / forwarded-IP the caller sent),
//! so each field is length-bounded before insert and the recorder swallows its
//! own errors — auditing must never break the auth flow it observes.

use super::session::now;
use super::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{FromRequestParts, State};
use axum::http::header::USER_AGENT;
use axum::http::request::Parts;
use axum::Json;
use sqlx::PgPool;
use std::convert::Infallible;
use syle_types::{AccessAction, AccessLogEntry, AccessMethod, AccessOutcome};
use uuid::Uuid;

const EMAIL_MAX: usize = 320;
const IP_MAX: usize = 64;
const UA_MAX: usize = 256;
/// Cap the read view; the trail is unbounded but the operator only needs recent.
const LIST_LIMIT: i64 = 200;

/// Truncate to at most `max` chars on a char boundary (never splits UTF-8).
fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Best-effort request provenance for the log. Infallible: a missing header
/// just yields `None`. IP is read from the proxy chain (Cloudflare → nginx) and
/// is **untrusted** — for display only, never for any access decision.
pub struct ClientMeta {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

impl<S: Send + Sync> FromRequestParts<S> for ClientMeta {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Infallible> {
        let h = &parts.headers;
        // Cloudflare's per-request visitor IP first; else the leftmost
        // X-Forwarded-For hop nginx appends; else X-Real-IP.
        let ip = h
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .or_else(|| {
                h.get("x-forwarded-for")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.split(',').next())
                    .map(str::trim)
            })
            .or_else(|| h.get("x-real-ip").and_then(|v| v.to_str().ok()).map(str::trim))
            .filter(|s| !s.is_empty())
            .map(|s| clip(s, IP_MAX));
        let user_agent = h
            .get(USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| clip(s, UA_MAX));
        Ok(ClientMeta { ip, user_agent })
    }
}

/// One audited event, minus the request metadata (carried by [`ClientMeta`]).
pub(crate) struct AccessEvent {
    pub user_id: Option<Uuid>,
    pub email: Option<String>,
    pub action: AccessAction,
    pub method: Option<AccessMethod>,
    pub outcome: AccessOutcome,
}

/// Append an event. Infallible by design: a logging failure is swallowed so it
/// can never take down login. Call it on the success path *before* any further
/// fallible work (e.g. minting the session) so the trail survives a later error.
pub(crate) async fn record(pool: &PgPool, ev: AccessEvent, meta: &ClientMeta) {
    let email = ev.email.map(|e| clip(&e, EMAIL_MAX));
    let _ = sqlx::query(
        "INSERT INTO access_log \
         (id, at, user_id, email, action, method, outcome, ip, user_agent) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(now())
    .bind(ev.user_id)
    .bind(email)
    .bind(ev.action.as_db_str())
    .bind(ev.method.map(AccessMethod::as_db_str))
    .bind(ev.outcome.as_db_str())
    .bind(meta.ip.as_deref())
    .bind(meta.user_agent.as_deref())
    .execute(pool)
    .await;
}

type Row = (
    Uuid,
    i64,
    Option<String>,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

/// Read-only audit trail, most recent first. Any operator session may read it;
/// the log is a single shared security record, not per-account.
pub async fn list_access_log(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
) -> Result<Json<Vec<AccessLogEntry>>, ApiError> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, at, email, action, method, outcome, ip, user_agent \
         FROM access_log ORDER BY seq DESC LIMIT $1",
    )
    .bind(LIST_LIMIT)
    .fetch_all(&state.pool)
    .await?;

    let entries = rows
        .into_iter()
        .map(|(id, at, email, action, method, outcome, ip, user_agent)| AccessLogEntry {
            id,
            at,
            email,
            // We are the only writer, so these always parse; default defensively.
            action: AccessAction::from_db_str(&action).unwrap_or(AccessAction::Login),
            method: method.as_deref().and_then(AccessMethod::from_db_str),
            outcome: AccessOutcome::from_db_str(&outcome).unwrap_or(AccessOutcome::Failure),
            ip,
            user_agent,
        })
        .collect();
    Ok(Json(entries))
}
