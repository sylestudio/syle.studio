use crate::ui::{ToastHost, Toaster};
use crate::views::{
    AccessLog, Dashboard, GalleriesView, GalleryView, Login, PostEditor, PostsView, ProjectView,
    ProjectsView, Recovery, Security, StudioShell,
};
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
                <Route path=path!("/recovery") view=Recovery />
                <ParentRoute path=path!("") view=StudioShell>
                    <Route path=path!("/galleries") view=GalleriesView />
                    <Route path=path!("/galleries/:id") view=GalleryView />
                    <Route path=path!("/projects") view=ProjectsView />
                    <Route path=path!("/projects/:id") view=ProjectView />
                    <Route path=path!("/posts") view=PostsView />
                    <Route path=path!("/posts/:id") view=PostEditor />
                    <Route path=path!("/security") view=Security />
                    <Route path=path!("/access-log") view=AccessLog />
                    <Route path=path!("/") view=Dashboard />
                </ParentRoute>
            </Routes>
        </Router>
        <ToastHost />
    }
}
