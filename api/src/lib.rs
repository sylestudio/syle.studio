//! syle.studio API library: route table + handlers.
//! `/api/public/*` is cacheable & unauthenticated; `/api/admin/*` is session
//! authenticated and only reachable through the Cloudflare Tunnel.

mod admin;
pub mod auth;
mod error;
mod public;
mod state;

pub use state::AppState;

use axum::routing::{get, post};
use axum::Router;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/public/galleries", get(public::list_galleries))
        .route("/api/public/galleries/{slug}", get(public::get_gallery))
        .route("/api/public/posts", get(public::list_posts))
        .route("/api/public/posts/{slug}", get(public::get_post))
        .route("/api/admin/galleries", post(admin::create_gallery))
        .route("/api/admin/photos", post(admin::upload_photo))
        .route("/api/admin/posts", post(admin::create_post))
        .route("/api/admin/login", post(auth::login))
        .route("/api/admin/logout", post(auth::logout))
        .route("/api/admin/me", get(auth::me))
        .with_state(state)
}
