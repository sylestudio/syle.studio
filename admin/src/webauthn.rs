//! Passkey ceremony orchestration: glue the JS shim (`/js/webauthn.js`) to the
//! API. Each flow is start → browser ceremony → finish; the shim returns the
//! opaque signed credential JSON which we forward to the server unaltered.

use crate::api::{self, ApiError};
use serde_json::Value;
use syle_types::{CredentialInfo, RecoveryRedeem, User, WebauthnFinish, WebauthnStart};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/webauthn.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn register(options: String) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn authenticate(options: String) -> Result<JsValue, JsValue>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PasskeyError {
    /// The user dismissed the OS prompt, or no usable credential was found.
    Cancelled,
    /// The API rejected the ceremony or the request failed.
    Api(ApiError),
}

/// The shim returns a JSON string; turn it into an opaque `Value` for the API.
fn shim_json(v: JsValue) -> Result<Value, PasskeyError> {
    let s = v.as_string().ok_or(PasskeyError::Cancelled)?;
    serde_json::from_str(&s).map_err(|_| PasskeyError::Cancelled)
}

/// Enroll a new passkey on the signed-in account.
pub async fn enroll() -> Result<CredentialInfo, PasskeyError> {
    let challenge = api::webauthn_register_start().await.map_err(PasskeyError::Api)?;
    let options = challenge.options.to_string();
    let credential = register(options).await.map_err(|_| PasskeyError::Cancelled)?;
    let credential = shim_json(credential)?;
    api::webauthn_register_finish(&WebauthnFinish {
        flow_id: challenge.flow_id,
        credential,
    })
    .await
    .map_err(PasskeyError::Api)
}

/// Email-first passkey login.
pub async fn login(email: String) -> Result<User, PasskeyError> {
    let challenge = api::webauthn_login_start(&WebauthnStart { email })
        .await
        .map_err(PasskeyError::Api)?;
    let options = challenge.options.to_string();
    let credential = authenticate(options).await.map_err(|_| PasskeyError::Cancelled)?;
    let credential = shim_json(credential)?;
    api::webauthn_login_finish(&WebauthnFinish {
        flow_id: challenge.flow_id,
        credential,
    })
    .await
    .map_err(PasskeyError::Api)
}

/// Redeem a recovery code (pre-auth). No browser ceremony — straight to the API.
pub async fn redeem(email: String, code: String) -> Result<User, PasskeyError> {
    api::recovery_redeem(&RecoveryRedeem { email, code })
        .await
        .map_err(PasskeyError::Api)
}
