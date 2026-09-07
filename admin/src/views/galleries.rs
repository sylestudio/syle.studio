use crate::api::{self, ApiError};
use crate::ui::{use_toaster, Badge, Button, Field, Heading, Skeleton, SlideOver, Tile, INPUT};
use icondata::HiPhotoOutlineLg;
use leptos::prelude::*;
use leptos_icons::Icon as HeroIcon;
use leptos_router::hooks::use_navigate;
use std::collections::HashMap;
use syle_types::{is_valid_slug, Gallery, NewGallery, Photo};
use wasm_bindgen_futures::spawn_local;

type Galleries = RwSignal<Vec<Gallery>>;
/// Gallery id to cover image path (absent while loading, `None` without photos).
type Covers = RwSignal<HashMap<String, Option<String>>>;

fn cover_src(photo: &Photo) -> String {
    photo
        .variants
        .iter()
        .filter(|variant| variant.path.ends_with(".jpeg"))
        .min_by_key(|variant| variant.width)
        .or_else(|| photo.variants.first())
        .map(|variant| variant.path.clone())
        .unwrap_or_default()
}

fn reload(galleries: Galleries, covers: Covers, auth_failed: RwSignal<bool>) {
    spawn_local(async move {
        match api::list_galleries().await {
            Ok(items) => {
                covers.set(HashMap::new());
                for gallery in &items {
                    let id = gallery.id;
                    spawn_local(async move {
                        if let Ok(detail) = api::gallery_detail(&id.to_string()).await {
                            let cover = detail
                                .photos
                                .first()
                                .map(cover_src)
                                .filter(|src| !src.is_empty());
                            covers.update(|items| {
                                items.insert(id.to_string(), cover);
                            });
                        }
                    });
                }
                galleries.set(items);
            }
            Err(ApiError::Unauthorized) => auth_failed.set(true),
            Err(_) => {}
        }
    });
}

#[component]
fn GalleryCard(gallery: Gallery, covers: Covers) -> impl IntoView {
    let id = gallery.id.to_string();
    let href = format!("/galleries/{id}");
    let published = gallery.published;

    view! {
        <a href=href class="group reveal-item block">
            <Tile>
                <div class="relative aspect-4/3 bg-zinc-800">
                    {move || match covers.get().get(&id) {
                        Some(Some(src)) => view! {
                            <img
                                src=src.clone()
                                alt=""
                                class="absolute inset-0 block h-full w-full object-cover \
                                    transition duration-500 ease-fluid group-hover:scale-105 \
                                    motion-reduce:transition-none \
                                    motion-reduce:group-hover:scale-100"
                            />
                        }.into_any(),
                        Some(None) => view! {
                            <div class="flex h-full w-full flex-col items-center \
                                justify-center gap-2 text-zinc-600">
                                <span class="flex size-8" aria-hidden="true">
                                    <HeroIcon icon=HiPhotoOutlineLg width="2rem" height="2rem" />
                                </span>
                                <span class="text-xs/5">"Sin fotos"</span>
                            </div>
                        }.into_any(),
                        None => view! { <Skeleton class="absolute inset-0" /> }.into_any(),
                    }}
                </div>
                <div class="flex items-center justify-between gap-3 px-4 py-3">
                    <span class="truncate text-sm/6 font-medium text-white \
                        transition-colors group-hover:text-zinc-300">
                        {gallery.title}
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
pub fn GalleriesView() -> impl IntoView {
    let galleries: Galleries = RwSignal::new(Vec::new());
    let covers: Covers = RwSignal::new(HashMap::new());
    let auth_failed = RwSignal::new(false);
    let goto = RwSignal::new(None::<String>);
    let navigate = use_navigate();

    Effect::new(move |_| reload(galleries, covers, auth_failed));
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

    let open = RwSignal::new(false);
    let slug = RwSignal::new(String::new());
    let title = RwSignal::new(String::new());
    let saving = RwSignal::new(false);
    let toast = use_toaster();
    let valid =
        Signal::derive(move || is_valid_slug(slug.get().trim()) && !title.get().trim().is_empty());
    let create = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if !valid.get() || saving.get() {
            toast.err("Falta el título o el slug no es válido");
            return;
        }
        let gallery = NewGallery {
            slug: slug.get().trim().into(),
            title: title.get().trim().into(),
            position: 0,
            published: false,
        };
        saving.set(true);
        spawn_local(async move {
            match api::create_gallery(&gallery).await {
                Ok(gallery) => {
                    toast.ok("Galería creada");
                    open.set(false);
                    goto.set(Some(format!("/galleries/{}", gallery.id)));
                }
                Err(_) => toast.err("No se pudo crear la galería"),
            }
            saving.set(false);
        });
    };

    view! {
        <div class="space-y-8">
            <div class="flex flex-wrap items-end justify-between gap-4">
                <div class="space-y-1">
                    <Heading text="Galerías" />
                    <p class="text-sm/6 text-zinc-400">
                        "Colecciones fotográficas publicadas en el portafolio."
                    </p>
                </div>
                <Button on:click=move |_| open.set(true)>"Nueva galería"</Button>
            </div>

            {move || {
                let items = galleries.get();
                if items.is_empty() {
                    view! {
                        <p class="text-sm/6 text-zinc-500">
                            "Aún no hay galerías. Crea la primera."
                        </p>
                    }.into_any()
                } else {
                    view! {
                        <div class="grid grid-cols-2 gap-5 sm:grid-cols-3 lg:grid-cols-4">
                            {items.into_iter().map(|gallery| view! {
                                <GalleryCard gallery=gallery covers=covers />
                            }).collect_view()}
                        </div>
                    }.into_any()
                }
            }}

            <SlideOver open=open title="Nueva galería">
                <form on:submit=create class="space-y-4">
                    <Field label="Título">
                        <input class=INPUT prop:value=title
                            on:input=move |event| title.set(event_target_value(&event)) />
                    </Field>
                    <Field label="Slug">
                        <input class=INPUT prop:value=slug
                            on:input=move |event| slug.set(event_target_value(&event)) />
                        <p class="text-xs/5 text-zinc-500">
                            "Minúsculas, números y guiones."
                        </p>
                    </Field>
                    <p class="text-xs/5 text-zinc-500">
                        "Se abrirá la galería para subir fotos."
                    </p>
                    <Button disabled=Signal::derive(move || !valid.get() || saving.get())>
                        {move || if saving.get() { "Creando…" } else { "Crear galería" }}
                    </Button>
                </form>
            </SlideOver>
        </div>
    }
}
