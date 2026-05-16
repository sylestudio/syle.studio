//! The persistent "studio" shell: a dark Catalyst sidebar-layout that wraps
//! every authenticated route via `<Outlet/>`. Visual classes are ported from
//! Catalyst `sidebar-layout`/`sidebar`; Headless-UI state is dropped (no
//! mobile drawer yet — the sidebar is always visible from `lg` up).

use crate::api;
use crate::views::CommandPalette;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::{use_location, use_navigate};
use wasm_bindgen_futures::spawn_local;

const ITEM: &str = "relative flex w-full items-center gap-3 rounded-lg px-2 py-2 \
    text-left text-sm/5 font-medium text-zinc-400 hover:bg-white/5 \
    hover:text-white transition duration-200 ease-fluid";
const ITEM_CURRENT: &str = "bezel-core relative flex w-full items-center gap-3 \
    rounded-lg px-2 py-2 text-left text-sm/5 font-medium text-white bg-white/5 \
    transition duration-200 ease-fluid";

#[component]
fn NavItem(
    #[prop(into)] href: String,
    #[prop(into)] label: String,
    /// Mark current when the path starts with this prefix ("/" = exact).
    #[prop(into)]
    matches: String,
) -> impl IntoView {
    let loc = use_location();
    let current = Signal::derive(move || {
        let p = loc.pathname.get();
        if matches == "/" {
            p == "/"
        } else {
            p.starts_with(&matches)
        }
    });
    view! {
        <a href=href class=move || if current.get() { ITEM_CURRENT } else { ITEM }>
            {move || current.get().then(|| view! {
                <span class="absolute inset-y-1.5 -left-4 w-0.5 rounded-full bg-white"></span>
            })}
            <span class="truncate">{label}</span>
        </a>
    }
}

#[component]
pub fn StudioShell() -> impl IntoView {
    let email = RwSignal::new(String::new());
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(u) = api::me().await {
                email.set(u.email);
            }
        });
    });

    let palette = RwSignal::new(false);
    let loc = use_location();

    let navigate = use_navigate();
    let logout = move |_| {
        let navigate = navigate.clone();
        spawn_local(async move {
            api::logout().await;
            navigate("/login", Default::default());
        });
    };

    view! {
        <div class="relative isolate flex min-h-svh w-full bg-zinc-950">
            // Film-grain texture layer (fixed, inert, below overlays)
            <div class="studio-grain"></div>
            // Fixed dark sidebar
            <div class="fixed inset-y-0 left-0 w-64 max-lg:hidden">
                <nav class="flex h-full min-h-0 flex-col">
                    <div class="flex flex-col border-b border-white/5 p-5">
                        <span class="text-base/6 font-semibold tracking-tight text-white">
                            "syle"<span class="text-zinc-500">".studio"</span>
                        </span>
                        <span class="text-xs/5 text-zinc-500">"estudio · CRM"</span>
                    </div>
                    <div class="flex flex-1 flex-col overflow-y-auto p-4">
                        <button
                            class="mb-6 flex items-center justify-between rounded-lg \
                                border border-white/10 bg-white/5 px-3 py-2 text-sm/5 \
                                text-zinc-400 hover:bg-white/10 hover:text-white \
                                transition duration-200 ease-fluid active:scale-[0.98] \
                                motion-reduce:transition-none motion-reduce:active:scale-100"
                            on:click=move |_| palette.set(true)
                        >
                            <span>"Buscar…"</span>
                            <kbd class="rounded border border-white/10 bg-white/5 px-1.5 \
                                py-0.5 text-xs text-zinc-500">"⌘K"</kbd>
                        </button>
                        <h3 class="mb-1 px-2 text-xs/6 font-medium text-zinc-500">
                            "Trabajo"
                        </h3>
                        <div class="flex flex-col gap-0.5">
                            <NavItem href="/" label="Portafolio" matches="/" />
                            <NavItem href="/" label="Galerías" matches="/galleries" />
                            <NavItem href="/" label="Blog" matches="/posts" />
                        </div>
                    </div>
                    <div class="flex flex-col gap-2 border-t border-white/5 p-4">
                        <span class="truncate px-2 text-xs/5 text-zinc-500">
                            {move || email.get()}
                        </span>
                        <button
                            class="rounded-lg px-2 py-2 text-left text-sm/5 font-medium \
                                text-zinc-400 hover:bg-white/5 hover:text-white transition-colors"
                            on:click=logout
                        >
                            "Salir"
                        </button>
                    </div>
                </nav>
            </div>

            // Content panel
            <main class="flex flex-1 flex-col pb-2 lg:min-w-0 lg:pt-2 lg:pr-2 lg:pl-64">
                <div class="grow p-6 lg:rounded-2xl lg:bg-zinc-900 lg:p-10 \
                    lg:shadow-sm lg:ring-1 lg:ring-white/10">
                    <div class="mx-auto max-w-6xl">
                        // Re-mount on pathname change so the CSS fade-up
                        // re-triggers on every route navigation.
                        {move || {
                            let _ = loc.pathname.get();
                            view! { <div class="route-fade"><Outlet /></div> }
                        }}
                    </div>
                </div>
            </main>

            <CommandPalette open=palette />
        </div>
    }
}
