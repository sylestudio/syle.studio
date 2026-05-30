//! Public-site rebuild status — the contract between the API (which talks to the
//! CI that rebuilds the static Astro site) and the CRM progress UI.

use serde::{Deserialize, Serialize};

/// Where the public-site rebuild stands. Mapped server-side from the CI run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteBuildState {
    /// No rebuild trigger is configured on the server (missing token) — the
    /// feature is dormant and the CRM hides/disables the control.
    Unconfigured,
    /// Configured, nothing relevant in flight.
    Idle,
    /// Dispatched; waiting for a runner to pick it up.
    Queued,
    /// A runner is building + publishing the site.
    Building,
    /// Last run finished successfully.
    Done,
    /// Last run finished with a failure.
    Failed,
}

/// Status of the most recent public-site rebuild.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteBuildStatus {
    pub state: SiteBuildState,
    /// Link to the CI run, for the operator to inspect logs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_url: Option<String>,
    /// CI run number — lets the CRM tell one run from the next.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_number: Option<u32>,
}

impl SiteBuildStatus {
    /// A bare status with no associated run.
    pub fn bare(state: SiteBuildState) -> Self {
        Self { state, run_url: None, run_number: None }
    }

    /// True while a rebuild is dispatched or running.
    pub fn is_active(&self) -> bool {
        matches!(self.state, SiteBuildState::Queued | SiteBuildState::Building)
    }
}
