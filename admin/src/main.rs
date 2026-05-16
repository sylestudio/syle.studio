//! syle CRM — Leptos CSR SPA for admin.syle.studio (served via Cloudflare
//! Tunnel, never publicly exposed).

use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <main>
            <h1>"syle.studio · CRM"</h1>
            <p>"Scaffold. Auth + gallery management land in task #6."</p>
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
