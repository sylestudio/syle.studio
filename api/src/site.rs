//! Public-site rebuild: trigger the static Astro rebuild via the GitHub Actions
//! API and report its status to the CRM progress UI.
//!
//! The site is `output: "static"` — content edits don't show until a rebuild.
//! `POST /rebuild` fires a `repository_dispatch` that runs `site.yml` on the
//! self-hosted runner (≈15s); `GET /status` maps the latest run for the CRM.
//!
//! The GitHub token lives server-side only (env, never in responses or logs).
//! HTTP responses stay deliberately terse (status code only); on failure we log
//! GitHub's status + a capped body snippet to journald so a mute 500 stays
//! debuggable from `journalctl -u syle-api` without ever touching the token.

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use syle_types::{SiteBuildState, SiteBuildStatus};

const GH_API: &str = "https://api.github.com";
/// The workflow file (on the default branch) that `repository_dispatch` runs.
const WORKFLOW: &str = "site.yml";
/// `repository_dispatch` event type `site.yml` listens for.
const EVENT_TYPE: &str = "rebuild-site";
/// How much of a GitHub error body we echo into journald — enough to see
/// GitHub's `message` (e.g. "Resource not accessible by personal access token")
/// without flooding the log on a large/HTML response.
const LOG_BODY_CAP: usize = 300;

/// Server-side rebuild config. Holds the token; intentionally NOT `Debug` so it
/// can never be logged. `None` (missing env) leaves the feature dormant.
#[derive(Clone)]
pub struct GithubConfig {
    client: reqwest::Client,
    token: String,
    /// `owner/name`.
    repo: String,
}

impl GithubConfig {
    /// Build from `GITHUB_DISPATCH_TOKEN` + `GITHUB_REPO`; `None` if either is
    /// unset/empty so the API runs fine without the feature configured.
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("GITHUB_DISPATCH_TOKEN").ok().filter(|s| !s.is_empty())?;
        let repo = std::env::var("GITHUB_REPO").ok().filter(|s| !s.is_empty())?;
        // GitHub requires a User-Agent; rustls TLS verification is on by default.
        let client = reqwest::Client::builder().user_agent("syle-api").build().ok()?;
        Some(Self { client, token, repo })
    }
}

/// Build a journald line for a failed GitHub call. Pure + tested. The token is
/// never in scope here (it rides only the `Authorization` header), so it cannot
/// leak; we still cap the body so an oversized response can't flood the log.
fn diag(context: &str, status: u16, body: &str) -> String {
    let body = body.trim();
    let snippet: String = body.chars().take(LOG_BODY_CAP).collect();
    let ellipsis = if body.chars().count() > LOG_BODY_CAP { "…" } else { "" };
    format!("site rebuild: {context} → GitHub HTTP {status}: {snippet}{ellipsis}")
}

/// Map a GitHub Actions run's `(status, conclusion)` to our state. Pure — the
/// tested core of this module.
fn map_run(status: &str, conclusion: Option<&str>) -> SiteBuildState {
    match status {
        "queued" | "waiting" | "pending" | "requested" => SiteBuildState::Queued,
        "in_progress" => SiteBuildState::Building,
        "completed" => match conclusion {
            Some("success") => SiteBuildState::Done,
            _ => SiteBuildState::Failed,
        },
        // Unknown/future statuses: treat as not-yet-done so the UI keeps polling.
        _ => SiteBuildState::Queued,
    }
}

/// Fetch the most recent `site.yml` run and map it.
async fn latest_run(cfg: &GithubConfig) -> Result<SiteBuildStatus, ()> {
    let url = format!("{GH_API}/repos/{}/actions/workflows/{WORKFLOW}/runs?per_page=1", cfg.repo);
    let resp = cfg
        .client
        .get(&url)
        .bearer_auth(&cfg.token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| eprintln!("site rebuild: status request failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        eprintln!("{}", diag("status", status.as_u16(), &body));
        return Err(());
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| eprintln!("site rebuild: status decode failed: {e}"))?;
    let Some(run) = body.get("workflow_runs").and_then(|r| r.get(0)) else {
        // Configured but the workflow has never run yet.
        return Ok(SiteBuildStatus::bare(SiteBuildState::Idle));
    };
    let status = run.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let conclusion = run.get("conclusion").and_then(|v| v.as_str());
    Ok(SiteBuildStatus {
        state: map_run(status, conclusion),
        run_url: run.get("html_url").and_then(|v| v.as_str()).map(String::from),
        run_number: run.get("run_number").and_then(|v| v.as_u64()).map(|n| n as u32),
    })
}

/// Fire the `repository_dispatch` that starts `site.yml`.
async fn dispatch(cfg: &GithubConfig) -> Result<(), ()> {
    let url = format!("{GH_API}/repos/{}/dispatches", cfg.repo);
    let resp = cfg
        .client
        .post(&url)
        .bearer_auth(&cfg.token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&serde_json::json!({ "event_type": EVENT_TYPE }))
        .send()
        .await
        .map_err(|e| eprintln!("site rebuild: dispatch request failed: {e}"))?;
    let status = resp.status();
    if status.is_success() {
        eprintln!("site rebuild: dispatched repository_dispatch ({EVENT_TYPE})");
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        eprintln!("{}", diag("dispatch", status.as_u16(), &body));
        Err(())
    }
}

/// `GET /api/admin/site/status` — current rebuild status for the CRM poll loop.
/// Never errors: unconfigured → `Unconfigured`, GitHub unreachable → `Idle`
/// (operator can still retry), so the UI degrades gracefully.
pub async fn site_status(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Json<SiteBuildStatus> {
    let Some(cfg) = state.github.as_ref() else {
        return Json(SiteBuildStatus::bare(SiteBuildState::Unconfigured));
    };
    match latest_run(cfg).await {
        Ok(s) => Json(s),
        Err(()) => Json(SiteBuildStatus::bare(SiteBuildState::Idle)),
    }
}

/// `POST /api/admin/site/rebuild` — trigger a rebuild. Debounced: 409 if one is
/// already queued/building, so spamming the button can't stack runs.
pub async fn rebuild_site(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<SiteBuildStatus>, ApiError> {
    let cfg = state.github.as_ref().ok_or(ApiError::ServiceUnavailable)?;
    if let Ok(cur) = latest_run(cfg).await {
        if cur.is_active() {
            return Err(ApiError::Conflict);
        }
    }
    dispatch(cfg).await.map_err(|_| ApiError::Internal)?;
    Ok(Json(SiteBuildStatus::bare(SiteBuildState::Queued)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_variants_map_to_queued() {
        for s in ["queued", "waiting", "pending", "requested"] {
            assert_eq!(map_run(s, None), SiteBuildState::Queued, "status {s}");
        }
    }

    #[test]
    fn in_progress_maps_to_building() {
        assert_eq!(map_run("in_progress", None), SiteBuildState::Building);
    }

    #[test]
    fn completed_success_maps_to_done() {
        assert_eq!(map_run("completed", Some("success")), SiteBuildState::Done);
    }

    #[test]
    fn completed_non_success_maps_to_failed() {
        for c in [Some("failure"), Some("cancelled"), Some("timed_out"), Some("startup_failure"), None] {
            assert_eq!(map_run("completed", c), SiteBuildState::Failed, "conclusion {c:?}");
        }
    }

    #[test]
    fn unknown_status_keeps_polling_as_queued() {
        assert_eq!(map_run("some_future_status", None), SiteBuildState::Queued);
    }

    #[test]
    fn is_active_only_for_queued_and_building() {
        assert!(SiteBuildStatus::bare(SiteBuildState::Queued).is_active());
        assert!(SiteBuildStatus::bare(SiteBuildState::Building).is_active());
        assert!(!SiteBuildStatus::bare(SiteBuildState::Done).is_active());
        assert!(!SiteBuildStatus::bare(SiteBuildState::Failed).is_active());
        assert!(!SiteBuildStatus::bare(SiteBuildState::Idle).is_active());
        assert!(!SiteBuildStatus::bare(SiteBuildState::Unconfigured).is_active());
    }

    #[test]
    fn diag_includes_context_status_and_body() {
        let line = diag(
            "dispatch",
            403,
            r#"{"message":"Resource not accessible by personal access token"}"#,
        );
        assert!(line.contains("dispatch"), "{line}");
        assert!(line.contains("403"), "{line}");
        assert!(
            line.contains("Resource not accessible by personal access token"),
            "{line}"
        );
    }

    #[test]
    fn diag_truncates_long_body_with_ellipsis() {
        let body = "x".repeat(LOG_BODY_CAP + 50);
        let line = diag("status", 500, &body);
        assert!(line.ends_with('…'), "{line}");
        // The body is capped, not echoed whole.
        assert_eq!(line.chars().filter(|&c| c == 'x').count(), LOG_BODY_CAP);
    }

    #[test]
    fn diag_short_body_has_no_ellipsis() {
        let line = diag("dispatch", 422, "boom");
        assert!(!line.contains('…'), "{line}");
        assert!(line.ends_with("boom"), "{line}");
    }
}
