-- Access log: an append-only audit trail of authentication events (password,
-- passkey and recovery-code logins, plus logout and security-management actions)
-- surfaced read-only in the CRM. Everything except `action`/`outcome` may be
-- attacker-controlled (a pre-auth login carries whatever email/User-Agent the
-- caller sent), so callers bound each field's length before insert and the UI
-- renders every value as escaped text.
CREATE TABLE access_log (
    id         UUID PRIMARY KEY,
    -- Monotonic insert order. `at` is seconds-granular, so two events in the
    -- same second tie; `seq` gives a total order for stable "most recent first".
    seq        BIGINT GENERATED ALWAYS AS IDENTITY,
    at         BIGINT NOT NULL,                                  -- unix seconds
    user_id    UUID REFERENCES users (id) ON DELETE SET NULL,    -- NULL if unresolved
    email      TEXT,                                             -- attempted email (untrusted)
    action     TEXT NOT NULL,                                    -- login | logout | passkey_enroll | passkey_revoke | recovery_generate
    method     TEXT,                                             -- password | passkey | recovery (login only)
    outcome    TEXT NOT NULL,                                    -- success | failure
    ip         TEXT,                                             -- best-effort client IP (untrusted)
    user_agent TEXT                                              -- best-effort User-Agent (untrusted)
);
CREATE INDEX access_log_seq_idx ON access_log (seq DESC);
