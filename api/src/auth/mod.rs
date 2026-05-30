//! Authentication. Today: Argon2id passwords + opaque DB sessions. Growing
//! additively into WebAuthn passkeys (public-key, phishing-resistant) and
//! single-use recovery codes, all minting the same `sid` session cookie.

mod audit;
mod credentials;
mod password;
mod recovery;
mod session;
mod webauthn;

pub use audit::{list_access_log, ClientMeta};
pub use credentials::{delete_credential, list_credentials, rename_credential};
pub use password::hash_password;
pub use recovery::{recovery_generate, recovery_redeem};
pub use session::{login, logout, me, AuthUser};
pub use webauthn::{
    build_webauthn, login_finish, login_start, register_finish, register_start,
};
