//! syle.studio API library: route table + handlers.
//! `/api/public/*` is cacheable & unauthenticated; `/api/admin/*` is session
//! authenticated and only reachable through the Cloudflare Tunnel.

mod admin;
pub mod auth;
mod error;
mod gallery_row;
mod public;
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
        .route("/api/admin/login", post(auth::login))
        .route("/api/admin/logout", post(auth::logout))
        .route("/api/admin/me", get(auth::me))
        .with_state(state)
}
