-- Passkeys (WebAuthn) + recovery codes. Additive over password auth.
-- Password stays as a permanent fallback in v1, so it becomes optional: a user
-- may authenticate with a passkey alone.
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- One row per registered passkey. `passkey` holds the serde-serialized
-- webauthn-rs `Passkey` (public key + sign counter); `credential_id` mirrors
-- `passkey.cred_id()` for fast lookup on authentication.
CREATE TABLE webauthn_credentials (
    id            UUID PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    credential_id BYTEA NOT NULL UNIQUE,
    passkey       JSONB NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    created_at    BIGINT NOT NULL,
    last_used_at  BIGINT
);
CREATE INDEX webauthn_credentials_user_id_idx ON webauthn_credentials (user_id);

-- Ephemeral ceremony state parked between */start and */finish. Single-use and
-- short-lived (expires_at = now + 300s); */finish consumes the row atomically.
CREATE TABLE webauthn_flows (
    flow_id    TEXT PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('register', 'auth')),
    state      JSONB NOT NULL,
    expires_at BIGINT NOT NULL
);
CREATE INDEX webauthn_flows_expires_at_idx ON webauthn_flows (expires_at);

-- Single-use recovery codes. Only the Argon2id hash is stored, never plaintext.
-- used_at NULL = still redeemable; regeneration deletes the prior set.
CREATE TABLE recovery_codes (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    code_hash  TEXT NOT NULL,
    used_at    BIGINT,
    created_at BIGINT NOT NULL
);
CREATE INDEX recovery_codes_user_id_idx ON recovery_codes (user_id);
