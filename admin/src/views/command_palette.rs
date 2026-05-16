//! ⌘K / Ctrl+K command palette: fuzzy-jump across galleries and blog posts.
//! Mounted once inside `StudioShell`, so it's available on every authed route.
//! A global window keydown listener toggles it; arrow/enter drive selection.

use crate::api;
use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use syle_types::{BlogPost, Gallery, PostStatus};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, PartialEq)]
struct Item {
    kind: &'static str,
    label: String,
    href: String,
}

fn build_items(gs: &[Gallery], ps: &[BlogPost], q: &str) -> Vec<Item> {
    let q = q.trim().to_lowercase();
    let mut out = vec![Item {
        kind: "Ir a",
        label: "Portafolio".into(),
        href: "/".into(),
    }];
    out.extend(gs.iter().map(|g| Item {
        kind: "Galería",
        label: g.title.clone(),
        href: format!("/galleries/{}", g.id),
    }));
    out.extend(ps.iter().map(|p| Item {
        kind: if p.status == PostStatus::Published {
            "Entrada"
        } else {
            "Borrador"
        },
        label: p.title.clone(),
        href: format!("/posts/{}", p.id),
    }));
    if q.is_empty() {
        out
    } else {
        out.into_iter()
            .filter(|i| i.label.to_lowercase().contains(&q))
            .collect()
    }
}

#[component]
pub fn CommandPalette(open: RwSignal<bool>) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let sel = RwSignal::new(0usize);
    let galleries = RwSignal::new(Vec::<Gallery>::new());
    let posts = RwSignal::new(Vec::<BlogPost>::new());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let navigate = use_navigate();

    // Global ⌘K / Ctrl+K toggle.
    let handle = window_event_listener(ev::keydown, move |e| {
        if (e.meta_key() || e.ctrl_key()) && e.key().to_lowercase() == "k" {
            e.prevent_default();
            open.update(|o| *o = !*o);
        }
    });
    on_cleanup(move || handle.remove());

    // On open: refresh data, reset state, focus the search box.
    Effect::new(move |_| {
        if open.get() {
            query.set(String::new());
            sel.set(0);
            spawn_local(async move {
                if let Ok(v) = api::list_galleries().await {
                    galleries.set(v);
                }
                if let Ok(v) = api::list_posts().await {
                    posts.set(v);
                }
            });
            if let Some(el) = input_ref.get() {
                let _ = el.focus();
            }
        }
    });

    let results = Memo::new(move |_| {
        build_items(&galleries.get(), &posts.get(), &query.get())
    });

    let go = {
        let navigate = navigate.clone();
        move |href: String| {
            open.set(false);
            navigate(&href, Default::default());
        }
    };

    let on_key = {
        let go = go.clone();
        move |e: ev::KeyboardEvent| {
            let n = results.get().len();
            match e.key().as_str() {
                "ArrowDown" => {
                    e.prevent_default();
                    if n > 0 {
                        sel.update(|s| *s = (*s + 1).min(n - 1));
                    }
                }
                "ArrowUp" => {
                    e.prevent_default();
                    sel.update(|s| *s = s.saturating_sub(1));
                }
                "Enter" => {
                    if let Some(it) = results.get().get(sel.get()) {
                        go(it.href.clone());
                    }
                }
                "Escape" => open.set(false),
                _ => {}
            }
        }
    };

    view! {
        {move || open.get().then(|| {
            let go = go.clone();
            view! {
                <div class="fixed inset-0 z-50 flex items-start justify-center p-4 pt-[12vh]">
                    <div
                        class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                        on:click=move |_| open.set(false)
                    ></div>
                    <div class="relative w-full max-w-xl overflow-hidden rounded-2xl \
                        border border-white/10 bg-zinc-900/95 shadow-2xl backdrop-blur-xl">
                        <input
                            node_ref=input_ref
                            class="w-full border-b border-white/10 bg-transparent px-5 \
                                py-4 text-base text-white outline-none placeholder:text-zinc-500"
                            placeholder="Buscar galerías y entradas…"
                            prop:value=query
                            on:input=move |e| { query.set(event_target_value(&e)); sel.set(0); }
                            on:keydown=on_key.clone()
                        />
                        <div class="max-h-80 overflow-y-auto p-2">
                            {move || {
                                let items = results.get();
                                if items.is_empty() {
                                    view! {
                                        <p class="px-3 py-6 text-center text-sm/6 text-zinc-500">
                                            "Sin resultados"
                                        </p>
                                    }.into_any()
                                } else {
                                    let go = go.clone();
                                    items.into_iter().enumerate().map(|(i, it)| {
                                        let go = go.clone();
                                        let href = it.href.clone();
                                        let active = move || sel.get() == i;
                                        view! {
                                            <button
                                                class=move || if active() {
                                                    "flex w-full items-center gap-3 rounded-lg \
                                                     bg-white/10 px-3 py-2.5 text-left transition-colors"
                                                } else {
                                                    "flex w-full items-center gap-3 rounded-lg \
                                                     px-3 py-2.5 text-left hover:bg-white/5 transition-colors"
                                                }
                                                on:mouseenter=move |_| sel.set(i)
                                                on:click=move |_| go(href.clone())
                                            >
                                                <span class="w-16 shrink-0 text-xs/5 font-medium text-zinc-500">
                                                    {it.kind}
                                                </span>
                                                <span class="truncate text-sm/6 text-white">
                                                    {it.label}
                                                </span>
                                            </button>
                                        }
                                    }).collect_view().into_any()
                                }
                            }}
                        </div>
                        <div class="flex items-center gap-4 border-t border-white/10 px-5 py-2.5 \
                            text-xs/5 text-zinc-500">
                            <span>"↑↓ navegar"</span>
                            <span>"↵ abrir"</span>
                            <span>"esc cerrar"</span>
                        </div>
                    </div>
                </div>
            }
        })}
    }
}
