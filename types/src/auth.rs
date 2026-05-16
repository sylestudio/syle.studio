use serde::{Deserialize, Serialize};
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
