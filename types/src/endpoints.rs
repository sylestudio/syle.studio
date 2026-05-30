//! Canonical API paths, shared by the API router, the Leptos CRM and the
//! Astro site so no end can drift from another.

pub const LOGIN: &str = "/api/admin/login";
pub const LOGOUT: &str = "/api/admin/logout";
pub const ME: &str = "/api/admin/me";

pub const ADMIN_GALLERIES: &str = "/api/admin/galleries";
pub const ADMIN_POSTS: &str = "/api/admin/posts";
pub const ADMIN_PHOTOS: &str = "/api/admin/photos";
pub const ADMIN_BLOG_ASSETS: &str = "/api/admin/blog-assets";

// Passkeys (WebAuthn). login/* and recovery/redeem are pre-session; the rest
// require an authenticated operator.
pub const WEBAUTHN_LOGIN_START: &str = "/api/admin/webauthn/login/start";
pub const WEBAUTHN_LOGIN_FINISH: &str = "/api/admin/webauthn/login/finish";
pub const WEBAUTHN_REGISTER_START: &str = "/api/admin/webauthn/register/start";
pub const WEBAUTHN_REGISTER_FINISH: &str = "/api/admin/webauthn/register/finish";
pub const WEBAUTHN_CREDENTIALS: &str = "/api/admin/webauthn/credentials";
pub const RECOVERY_REDEEM: &str = "/api/admin/recovery/redeem";
pub const RECOVERY_GENERATE: &str = "/api/admin/recovery/generate";

/// Read-only audit trail of authentication events (authenticated).
pub const ADMIN_ACCESS_LOG: &str = "/api/admin/access-log";

pub const PUBLIC_GALLERIES: &str = "/api/public/galleries";
pub const PUBLIC_POSTS: &str = "/api/public/posts";

pub fn admin_gallery(id: &str) -> String {
    format!("{ADMIN_GALLERIES}/{id}")
}

pub fn admin_gallery_order(id: &str) -> String {
    format!("{ADMIN_GALLERIES}/{id}/photos/order")
}

pub fn admin_photo(id: &str) -> String {
    format!("{ADMIN_PHOTOS}/{id}")
}

pub fn admin_post(id: &str) -> String {
    format!("{ADMIN_POSTS}/{id}")
}

pub fn webauthn_credential(id: &str) -> String {
    format!("{WEBAUTHN_CREDENTIALS}/{id}")
}

pub fn public_gallery(slug: &str) -> String {
    format!("{PUBLIC_GALLERIES}/{slug}")
}

pub fn public_post(slug: &str) -> String {
    format!("{PUBLIC_POSTS}/{slug}")
}
