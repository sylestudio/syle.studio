use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// CRM operator. There is no public sign-up; accounts are provisioned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
}

/// Credentials submitted to the CRM login endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Opaque session token returned on successful login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub token: String,
    /// Expiry as Unix seconds.
    pub expires_at: i64,
}

/// Begin an email-first passkey login: the operator types their email, we
/// look up their credentials and return a challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebauthnStart {
    pub email: String,
}

/// A WebAuthn ceremony challenge. `options` is the opaque
/// `CreationChallengeResponse`/`RequestChallengeResponse` JSON the browser
/// shim feeds to `navigator.credentials`; `flow_id` pairs it with the
/// server-side ceremony state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowChallenge {
    pub flow_id: String,
    pub options: Value,
}

/// The browser's signed ceremony response, returned to finish register/login.
/// `credential` is the opaque `RegisterPublicKeyCredential`/`PublicKeyCredential`
/// JSON; the server validates it against the stored flow state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebauthnFinish {
    pub flow_id: String,
    pub credential: Value,
}

/// Redeem a single-use recovery code (pre-auth, mints a session).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRedeem {
    pub email: String,
    pub code: String,
}

/// Freshly generated recovery codes, returned to the operator exactly once.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryCodes {
    pub codes: Vec<String>,
}

/// A registered passkey as shown in the credential-management UI. The public
/// key and counter never leave the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

/// Give a passkey a human label (e.g. "MacBook Touch ID").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameCredential {
    pub name: String,
}

/// What an access-log row records. `Login` carries an [`AccessMethod`]; the
/// rest are method-less security events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessAction {
    Login,
    Logout,
    PasskeyEnroll,
    PasskeyRevoke,
    RecoveryGenerate,
}

/// How an access (`Login`) event authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessMethod {
    Password,
    Passkey,
    Recovery,
}

/// Whether the event succeeded. Failures are the security signal worth surfacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessOutcome {
    Success,
    Failure,
}

impl AccessAction {
    /// Stable text stored in the DB `action` column (decoupled from serde).
    pub fn as_db_str(self) -> &'static str {
        match self {
            AccessAction::Login => "login",
            AccessAction::Logout => "logout",
            AccessAction::PasskeyEnroll => "passkey_enroll",
            AccessAction::PasskeyRevoke => "passkey_revoke",
            AccessAction::RecoveryGenerate => "recovery_generate",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "login" => AccessAction::Login,
            "logout" => AccessAction::Logout,
            "passkey_enroll" => AccessAction::PasskeyEnroll,
            "passkey_revoke" => AccessAction::PasskeyRevoke,
            "recovery_generate" => AccessAction::RecoveryGenerate,
            _ => return None,
        })
    }
}

impl AccessMethod {
    pub fn as_db_str(self) -> &'static str {
        match self {
            AccessMethod::Password => "password",
            AccessMethod::Passkey => "passkey",
            AccessMethod::Recovery => "recovery",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "password" => AccessMethod::Password,
            "passkey" => AccessMethod::Passkey,
            "recovery" => AccessMethod::Recovery,
            _ => return None,
        })
    }
}

impl AccessOutcome {
    pub fn as_db_str(self) -> &'static str {
        match self {
            AccessOutcome::Success => "success",
            AccessOutcome::Failure => "failure",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "success" => AccessOutcome::Success,
            "failure" => AccessOutcome::Failure,
            _ => return None,
        })
    }
}

/// One row of the access log as shown read-only in the CRM. Every optional
/// string is best-effort and attacker-influenceable — render as escaped text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub id: Uuid,
    /// Unix seconds.
    pub at: i64,
    pub email: Option<String>,
    pub action: AccessAction,
    pub method: Option<AccessMethod>,
    pub outcome: AccessOutcome,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}
