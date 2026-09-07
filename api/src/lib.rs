//! syle.studio API library: route table + handlers.
//! `/api/public/*` is cacheable & unauthenticated; `/api/admin/*` is session
//! authenticated and only reachable through the Cloudflare Tunnel.

mod admin;
pub mod auth;
mod error;
mod gallery_row;
mod project_row;
mod public;
mod site;
mod state;

pub use state::AppState;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};
use axum::Router;
use tower_http::services::ServeDir;

/// Per-request cap for the photo upload route. Real shoots run 30–50MB;
/// 64MiB leaves headroom over Axum's 2MB default, which the route overrides.
pub(crate) const MAX_UPLOAD_BYTES: usize = 64 * 1024 * 1024;

pub fn app(state: AppState) -> Router {
    // Serve ingested derivatives. In prod the CDN/Caddy front this; the admin
    // origin proxies it so the CRM can preview thumbnails.
    // Files are written under `<media_dir>/media/...` and `nest_service`
    // strips the `/media` prefix, so serve the inner directory.
    let media = ServeDir::new(state.media_dir.join("media"));
    Router::new()
        .nest_service("/media", media)
        .route("/api/public/galleries", get(public::list_galleries))
        .route("/api/public/galleries/{slug}", get(public::get_gallery))
        .route("/api/public/projects", get(public::list_projects))
        .route("/api/public/posts", get(public::list_posts))
        .route("/api/public/posts/{slug}", get(public::get_post))
        .route(
            "/api/admin/galleries",
            get(admin::list_galleries).post(admin::create_gallery),
        )
        .route(
            "/api/admin/galleries/{id}",
            get(admin::get_gallery_detail)
                .patch(admin::update_gallery)
                .delete(admin::delete_gallery),
        )
        .route(
            "/api/admin/galleries/{id}/photos/order",
            patch(admin::reorder_photos),
        )
        .route(
            "/api/admin/projects",
            get(admin::list_projects).post(admin::create_project),
        )
        .route(
            "/api/admin/projects/{id}",
            get(admin::get_project)
                .patch(admin::update_project)
                .delete(admin::delete_project),
        )
        .route(
            "/api/admin/photos/{id}",
            patch(admin::update_photo).delete(admin::delete_photo),
        )
        .route(
            "/api/admin/posts",
            get(admin::list_posts).post(admin::create_post),
        )
        .route(
            "/api/admin/posts/{id}",
            get(admin::get_post)
                .patch(admin::update_post)
                .delete(admin::delete_post),
        )
        .route(
            "/api/admin/photos",
            post(admin::upload_photo).layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES)),
        )
        .route(
            "/api/admin/blog-assets",
            post(admin::upload_blog_asset).layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES)),
        )
        .route(
            "/api/admin/project-assets",
            post(admin::upload_blog_asset).layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES)),
        )
        .route("/api/admin/login", post(auth::login))
        .route("/api/admin/logout", post(auth::logout))
        .route("/api/admin/me", get(auth::me))
        .route(
            "/api/admin/webauthn/login/start",
            post(auth::login_start),
        )
        .route(
            "/api/admin/webauthn/login/finish",
            post(auth::login_finish),
        )
        .route(
            "/api/admin/webauthn/register/start",
            post(auth::register_start),
        )
        .route(
            "/api/admin/webauthn/register/finish",
            post(auth::register_finish),
        )
        .route(
            "/api/admin/webauthn/credentials",
            get(auth::list_credentials),
        )
        .route(
            "/api/admin/webauthn/credentials/{id}",
            patch(auth::rename_credential).delete(auth::delete_credential),
        )
        .route("/api/admin/recovery/redeem", post(auth::recovery_redeem))
        .route(
            "/api/admin/recovery/generate",
            post(auth::recovery_generate),
        )
        .route("/api/admin/access-log", get(auth::list_access_log))
        .route("/api/admin/site/rebuild", post(site::rebuild_site))
        .route("/api/admin/site/status", get(site::site_status))
        .with_state(state)
}
