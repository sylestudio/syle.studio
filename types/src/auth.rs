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
