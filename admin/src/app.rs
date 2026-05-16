use crate::ui::{ToastHost, Toaster};
use crate::views::{Dashboard, GalleryView, Login, PostEditor, StudioShell};
use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    provide_context(Toaster::new());
    view! {
        <Router>
            <Routes fallback=|| view! { <p class="p-6">"No encontrado"</p> }>
                <Route path=path!("/login") view=Login />
                <ParentRoute path=path!("") view=StudioShell>
                    <Route path=path!("/galleries/:id") view=GalleryView />
                    <Route path=path!("/posts/:id") view=PostEditor />
                    <Route path=path!("/") view=Dashboard />
                </ParentRoute>
            </Routes>
        </Router>
        <ToastHost />
    }
}
