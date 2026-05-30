//! syle CRM — Leptos CSR SPA for admin.syle.studio (served via Cloudflare
//! Tunnel, never publicly exposed).

mod api;
mod app;
mod editor;
mod image_editor;
mod ui;
mod views;
mod webauthn;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
