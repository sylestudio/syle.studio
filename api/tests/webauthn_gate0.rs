//! Gate #0 — pure library round-trip, no router/HTTP/DB.
//!
//! Validates the riskiest premise of the whole passkeys feature *before* any
//! production code exists:
//!
//!   1. `webauthn-rs` and `webauthn-authenticator-rs` resolve at the SAME exact
//!      version. Each transitively pins `webauthn-rs-proto` exactly, so any
//!      drift fails `cargo build` here first — the earliest, loudest signal.
//!   2. The ceremony state types (`PasskeyRegistration`/`PasskeyAuthentication`)
//!      round-trip through JSONB. This only compiles with the
//!      `danger-allow-state-serialisation` feature; without it the whole
//!      "park ceremony state in Postgres between start/finish" design is dead.
//!   3. A full register -> authenticate round-trip succeeds against an
//!      in-process `SoftPasskey`, proving the API surface the handlers will use.
//!
//! NB: `SoftPasskey::new(true)` — `start_passkey_registration` pins
//! `UserVerificationPolicy::Required`, and the soft authenticator refuses
//! (`NotSupported`) unless it is allowed to falsify the UV flag. `new(false)`
//! would make the round-trip fail.

use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_rs::prelude::*;

const RP_ID: &str = "localhost";
const RP_ORIGIN: &str = "http://localhost:8080";

/// Serialize to a `serde_json::Value` and back — exactly what the JSONB
/// `state`/`passkey` columns will do between ceremony steps.
fn jsonb_round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_value(value).expect("serialize to JSONB");
    serde_json::from_value(json).expect("deserialize from JSONB")
}

#[test]
fn library_register_then_authenticate_round_trip() {
    // --- relying party (server side) ---
    let rp_origin = Url::parse(RP_ORIGIN).expect("valid rp origin");
    let webauthn = WebauthnBuilder::new(RP_ID, &rp_origin)
        .expect("builder")
        .rp_name("syle.studio CRM")
        .allow_subdomains(true)
        .build()
        .expect("build webauthn");

    // --- authenticator (client side stand-in) ---
    let origin = Url::parse(RP_ORIGIN).expect("valid origin");
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));

    // === registration ceremony ===
    let user_id = Uuid::new_v4();
    let (ccr, reg_state) = webauthn
        .start_passkey_registration(user_id, "op@syle.studio", "Operator", None)
        .expect("start registration");

    // The server would park `reg_state` in JSONB while the client signs.
    let reg_state = jsonb_round_trip(&reg_state);

    let reg_pkc = authenticator
        .do_registration(origin.clone(), ccr)
        .expect("authenticator registration");
    let passkey: Passkey = webauthn
        .finish_passkey_registration(&reg_pkc, &reg_state)
        .expect("finish registration");

    // Persisted as JSONB, reloaded for every future authentication.
    let passkey = jsonb_round_trip(&passkey);

    // === authentication ceremony ===
    let (rcr, auth_state) = webauthn
        .start_passkey_authentication(std::slice::from_ref(&passkey))
        .expect("start authentication");

    let auth_state = jsonb_round_trip(&auth_state);

    let auth_pkc = authenticator
        .do_authentication(origin, rcr)
        .expect("authenticator authentication");
    let result: AuthenticationResult = webauthn
        .finish_passkey_authentication(&auth_pkc, &auth_state)
        .expect("finish authentication");

    // The credential that just authenticated is the one we registered.
    assert_eq!(result.cred_id().as_ref(), passkey.cred_id().as_ref());
}
