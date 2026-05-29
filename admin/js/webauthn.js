// WebAuthn ceremony shim (0 dependencies). Loaded into wasm via
// `#[wasm_bindgen(module = "/js/webauthn.js")]`.
//
// Contract: each function takes the opaque options JSON produced by the server
// (a `{ "publicKey": { ... } }` object as a string) and returns the browser's
// signed response as a JSON string, which Rust forwards verbatim to the API.
//
// The shim DECIDES NOTHING about trust: origin, rp_id, challenge freshness and
// attestation are all validated server-side. It only marshals bytes between the
// base64url wire format and the `BufferSource`/`ArrayBuffer` the platform API
// wants. Native `*FromJSON()` / `toJSON()` are used when present (they ARE the
// spec wire format webauthn-rs speaks); otherwise a hand-rolled base64url path
// does the same marshalling. Errors (cancellation, InvalidState, …) are thrown
// and surface to Rust as a rejected promise.

function b64urlToBytes(s) {
  // URL-safe (RFC 4648 §5) → standard base64, re-padded; atob handles standard.
  const b64 = s.replace(/-/g, "+").replace(/_/g, "/");
  const pad = b64.length % 4 === 0 ? "" : "=".repeat(4 - (b64.length % 4));
  const bin = atob(b64 + pad);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

function bytesToB64url(buf) {
  const bytes = new Uint8Array(buf);
  let bin = "";
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
  // Strip padding, swap to URL-safe alphabet.
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function supportsNativeJson() {
  return (
    typeof PublicKeyCredential !== "undefined" &&
    typeof PublicKeyCredential.parseCreationOptionsFromJSON === "function" &&
    typeof PublicKeyCredential.parseRequestOptionsFromJSON === "function"
  );
}

// --- hand-rolled fallback marshalling --------------------------------------

function decodeCreationOptions(pk) {
  pk.challenge = b64urlToBytes(pk.challenge);
  pk.user.id = b64urlToBytes(pk.user.id);
  if (Array.isArray(pk.excludeCredentials)) {
    for (const c of pk.excludeCredentials) c.id = b64urlToBytes(c.id);
  }
  return pk;
}

function decodeRequestOptions(pk) {
  pk.challenge = b64urlToBytes(pk.challenge);
  if (Array.isArray(pk.allowCredentials)) {
    for (const c of pk.allowCredentials) c.id = b64urlToBytes(c.id);
  }
  return pk;
}

function encodeRegistration(cred) {
  const r = cred.response;
  const out = {
    id: cred.id,
    rawId: bytesToB64url(cred.rawId),
    type: cred.type,
    response: {
      attestationObject: bytesToB64url(r.attestationObject),
      clientDataJSON: bytesToB64url(r.clientDataJSON),
    },
    clientExtensionResults: cred.getClientExtensionResults
      ? cred.getClientExtensionResults()
      : {},
  };
  if (typeof r.getTransports === "function") {
    const t = r.getTransports();
    if (t && t.length) out.response.transports = t;
  }
  return out;
}

function encodeAuthentication(cred) {
  const r = cred.response;
  return {
    id: cred.id,
    rawId: bytesToB64url(cred.rawId),
    type: cred.type,
    response: {
      authenticatorData: bytesToB64url(r.authenticatorData),
      clientDataJSON: bytesToB64url(r.clientDataJSON),
      signature: bytesToB64url(r.signature),
      userHandle: r.userHandle ? bytesToB64url(r.userHandle) : null,
    },
    clientExtensionResults: cred.getClientExtensionResults
      ? cred.getClientExtensionResults()
      : {},
  };
}

// --- public API ------------------------------------------------------------

export async function register(optionsJson) {
  if (typeof PublicKeyCredential === "undefined") {
    throw new Error("WebAuthn unsupported");
  }
  const options = JSON.parse(optionsJson);
  const publicKey = supportsNativeJson()
    ? PublicKeyCredential.parseCreationOptionsFromJSON(options.publicKey)
    : decodeCreationOptions(options.publicKey);
  const cred = await navigator.credentials.create({ publicKey });
  const out = cred.toJSON ? cred.toJSON() : encodeRegistration(cred);
  return JSON.stringify(out);
}

export async function authenticate(optionsJson) {
  if (typeof PublicKeyCredential === "undefined") {
    throw new Error("WebAuthn unsupported");
  }
  const options = JSON.parse(optionsJson);
  const publicKey = supportsNativeJson()
    ? PublicKeyCredential.parseRequestOptionsFromJSON(options.publicKey)
    : decodeRequestOptions(options.publicKey);
  const cred = await navigator.credentials.get({ publicKey });
  const out = cred.toJSON ? cred.toJSON() : encodeAuthentication(cred);
  return JSON.stringify(out);
}
