use crate::api::{self, ApiError};
use crate::ui::{use_toaster, Badge, Button, Field, Heading, Skeleton, SlideOver, Tile, INPUT};
use crate::views::site_publish::SitePublish;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use std::collections::HashMap;
use syle_types::{is_valid_slug, BlogPost, Gallery, NewGallery, NewPost, Photo, PostStatus};

use wasm_bindgen_futures::spawn_local;

type Galleries = RwSignal<Vec<Gallery>>;
type Posts = RwSignal<Vec<BlogPost>>;
/// gallery id → its cover image path (absent = still loading, None = no photo).
type Covers = RwSignal<HashMap<String, Option<String>>>;

/// Smallest JPEG variant (any variant as fallback) for a cover thumbnail.
fn cover_src(p: &Photo) -> String {
    p.variants
        .iter()
        .filter(|v| v.path.ends_with(".jpeg"))
        .min_by_key(|v| v.width)
        .or_else(|| p.variants.first())
        .map(|v| v.path.clone())
        .unwrap_or_default()
}

fn reload(galleries: Galleries, posts: Posts, covers: Covers, auth_failed: RwSignal<bool>) {
    spawn_local(async move {
        if api::me().await == Err(ApiError::Unauthorized) {
            auth_failed.set(true);
            return;
        }
        if let Ok(v) = api::list_galleries().await {
            for g in &v {
                let id = g.id;
                spawn_local(async move {
                    if let Ok(d) = api::gallery_detail(&id.to_string()).await {
                        let cover = d
                            .photos
                            .first()
                            .map(cover_src)
                            .filter(|s| !s.is_empty());
                        covers.update(|m| {
                            m.insert(id.to_string(), cover);
                        });
                    }
                });
            }
            galleries.set(v);
        }
        if let Ok(v) = api::list_posts().await {
            posts.set(v);
        }
    });
}

#[component]
fn GalleryCard(g: Gallery, covers: Covers) -> impl IntoView {
    let id = g.id.to_string();
    let href = format!("/galleries/{id}");
    let published = g.published;
    view! {
        <a href=href class="group reveal-item block">
            <Tile>
                <div class="relative aspect-4/3 bg-zinc-800">
                    {move || match covers.get().get(&id) {
                        Some(Some(src)) => view! {
                            <img src=src.clone() alt=""
                                class="absolute inset-0 block h-full w-full object-cover \
                                    transition duration-500 ease-fluid group-hover:scale-105 \
                                    motion-reduce:transition-none motion-reduce:group-hover:scale-100" />
                        }.into_any(),
                        Some(None) => view! {
                            <div class="flex h-full w-full flex-col items-center justify-center gap-2 text-zinc-600">
                                <svg viewBox="0 0 24 24" fill="none" class="size-8"
                                    stroke="currentColor" stroke-width="1.5">
                                    <path stroke-linecap="round" stroke-linejoin="round"
                                        d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909M4.5 19.5h15a2.25 2.25 0 002.25-2.25V6.75A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25v10.5A2.25 2.25 0 004.5 19.5z" />
                                </svg>
                                <span class="text-xs/5">"Sin fotos"</span>
                            </div>
                        }.into_any(),
                        None => view! {
                            <Skeleton class="absolute inset-0" />
                        }.into_any(),
                    }}
                </div>
                <div class="flex items-center justify-between gap-3 px-4 py-3">
                    <span class="truncate text-sm/6 font-medium text-white \
                        transition-colors group-hover:text-zinc-300">
                        {g.title}
                    </span>
                    {if published {
                        view! { <Badge tone="green">"publicada"</Badge> }.into_any()
                    } else {
                        view! { <Badge>"borrador"</Badge> }.into_any()
                    }}
                </div>
            </Tile>
        </a>
    }
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let galleries: Galleries = RwSignal::new(Vec::new());
    let posts: Posts = RwSignal::new(Vec::new());
    let covers: Covers = RwSignal::new(HashMap::new());
    let auth_failed = RwSignal::new(false);
    let goto = RwSignal::new(None::<String>);
    let navigate = use_navigate();

    Effect::new(move |_| reload(galleries, posts, covers, auth_failed));
    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if auth_failed.get() {
                navigate("/login", Default::default());
            }
        }
    });
    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if let Some(path) = goto.get() {
                navigate(&path, Default::default());
            }
        }
    });

    let toast = use_toaster();
    let g_open = RwSignal::new(false);
    let p_open = RwSignal::new(false);

    let g_slug = RwSignal::new(String::new());
    let g_title = RwSignal::new(String::new());
    let g_valid = Signal::derive(move || {
        is_valid_slug(g_slug.get().trim()) && !g_title.get().trim().is_empty()
    });
    let create_gallery = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if !g_valid.get() {
            toast.err("Falta el título o el slug no es válido");
            return;
        }
        let n = NewGallery {
            slug: g_slug.get().trim().into(),
            title: g_title.get().trim().into(),
            position: 0,
            published: false,
        };
        spawn_local(async move {
            match api::create_gallery(&n).await {
                Ok(g) => {
                    g_slug.set(String::new());
                    g_title.set(String::new());
                    g_open.set(false);
                    toast.ok("Galería creada");
                    goto.set(Some(format!("/galleries/{}", g.id)));
                }
                Err(_) => toast.err("No se pudo crear la galería"),
            }
        });
    };

    let p_slug = RwSignal::new(String::new());
    let p_title = RwSignal::new(String::new());
    let p_valid = Signal::derive(move || {
        is_valid_slug(p_slug.get().trim()) && !p_title.get().trim().is_empty()
    });
    let create_post = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if !p_valid.get() {
            toast.err("Falta el título o el slug no es válido");
            return;
        }
        let n = NewPost {
            slug: p_slug.get().trim().into(),
            title: p_title.get().trim().into(),
            blocks: Vec::new(),
            status: PostStatus::Draft,
        };
        spawn_local(async move {
            match api::create_post(&n).await {
                Ok(p) => {
                    p_slug.set(String::new());
                    p_title.set(String::new());
                    p_open.set(false);
                    toast.ok("Borrador creado");
                    goto.set(Some(format!("/posts/{}", p.id)));
                }
                Err(_) => toast.err("No se pudo crear el borrador"),
            }
        });
    };

    view! {
        <div class="space-y-12">
            <div class="flex flex-wrap items-end justify-between gap-4">
                <div class="space-y-1">
                    <Heading text="Portafolio" />
                    <p class="text-sm/6 text-zinc-400">"Galerías y blog del estudio"</p>
                </div>
                <div class="flex gap-3">
                    <Button kind="outline" on:click=move |_| p_open.set(true)>
                        "Nueva entrada"
                    </Button>
                    <Button on:click=move |_| g_open.set(true)>"Nueva galería"</Button>
                </div>
            </div>

            <SitePublish />

            <section class="space-y-4">
                <h2 class="text-xs/6 font-medium tracking-wide text-zinc-500 uppercase">
                    "Galerías"
                </h2>
                {move || {
                    let gs = galleries.get();
                    if gs.is_empty() {
                        view! {
                            <p class="text-sm/6 text-zinc-500">
                                "Aún no hay galerías. Crea la primera."
                            </p>
                        }.into_any()
                    } else {
                        view! {
                            <div class="grid grid-cols-2 gap-5 sm:grid-cols-3 lg:grid-cols-4">
                                {gs.into_iter().map(|g| view! {
                                    <GalleryCard g=g covers=covers />
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                }}
            </section>

            <section class="space-y-4">
                <h2 class="text-xs/6 font-medium tracking-wide text-zinc-500 uppercase">
                    "Blog"
                </h2>
                {move || {
                    let ps = posts.get();
                    if ps.is_empty() {
                        view! {
                            <p class="text-sm/6 text-zinc-500">"Sin entradas todavía."</p>
                        }.into_any()
                    } else {
                        view! {
                            <div class="rounded-2xl border border-white/10 bg-white/4 p-1.5">
                            <ul class="bezel-core divide-y divide-white/5 overflow-hidden rounded-xl bg-zinc-900">
                                {ps.into_iter().map(|p| {
                                    let pub_ = p.status == PostStatus::Published;
                                    view! {
                                        <li class="reveal-item flex items-center justify-between gap-3 \
                                            px-5 py-4 transition-colors hover:bg-white/3">
                                            <a href=format!("/posts/{}", p.id)
                                                class="truncate text-sm/6 font-medium text-white \
                                                    transition-colors hover:text-zinc-300">
                                                {p.title}
                                            </a>
                                            {if pub_ {
                                                view! { <Badge tone="green">"publicado"</Badge> }.into_any()
                                            } else {
                                                view! { <Badge tone="amber">"borrador"</Badge> }.into_any()
                                            }}
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                            </div>
                        }.into_any()
                    }
                }}
            </section>

            <SlideOver open=g_open title="Nueva galería">
                <form on:submit=create_gallery class="space-y-4">
                    <Field label="Título">
                        <input class=INPUT prop:value=g_title
                            on:input=move |e| g_title.set(event_target_value(&e)) />
                    </Field>
                    <Field label="Slug">
                        <input class=INPUT prop:value=g_slug
                            on:input=move |e| g_slug.set(event_target_value(&e)) />
                        <p class="text-xs/5 text-zinc-500">
                            "Minúsculas, números y guiones."
                        </p>
                    </Field>
                    <p class="text-xs/5 text-zinc-500">
                        "Se abrirá la galería para subir fotos."
                    </p>
                    <Button disabled=Signal::derive(move || !g_valid.get())>
                        "Crear galería"
                    </Button>
                </form>
            </SlideOver>

            <SlideOver open=p_open title="Nueva entrada">
                <form on:submit=create_post class="space-y-4">
                    <Field label="Título">
                        <input class=INPUT prop:value=p_title
                            on:input=move |e| p_title.set(event_target_value(&e)) />
                    </Field>
                    <Field label="Slug">
                        <input class=INPUT prop:value=p_slug
                            on:input=move |e| p_slug.set(event_target_value(&e)) />
                        <p class="text-xs/5 text-zinc-500">
                            "Minúsculas, números y guiones."
                        </p>
                    </Field>
                    <p class="text-xs/5 text-zinc-500">
                        "Se abrirá el editor con vista previa para escribir el contenido."
                    </p>
                    <Button disabled=Signal::derive(move || !p_valid.get())>
                        "Crear borrador"
                    </Button>
                </form>
            </SlideOver>
        </div>
    }
}
