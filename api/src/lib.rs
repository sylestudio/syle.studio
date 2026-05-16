//! syle.studio API library: route table + handlers.
//! `/api/public/*` is cacheable & unauthenticated; `/api/admin/*` is session
//! authenticated and only reachable through the Cloudflare Tunnel.

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
        .route("/api/admin/login", post(auth::login))
        .route("/api/admin/logout", post(auth::logout))
        .route("/api/admin/me", get(auth::me))
        .with_state(state)
}
