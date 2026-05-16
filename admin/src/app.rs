use crate::views::{Dashboard, Login};
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <p class="p-6">"No encontrado"</p> }>
                <Route path=path!("/login") view=Login />
                <Route path=path!("/") view=Dashboard />
            </Routes>
        </Router>
    }
}
