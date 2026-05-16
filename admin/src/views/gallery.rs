use crate::api::{self, ApiError};
use crate::ui::{Button, Card, Field, Heading, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use syle_types::{GalleryDetail, Photo, Reorder, UpdateGallery, UpdatePhoto};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// Smallest JPEG variant (or any variant) for a grid thumbnail.
fn thumb_src(p: &Photo) -> String {
    p.variants
        .iter()
        .filter(|v| v.path.ends_with(".jpeg"))
        .min_by_key(|v| v.width)
        .or_else(|| p.variants.first())
        .map(|v| v.path.clone())
        .unwrap_or_default()
}

#[component]
pub fn GalleryView() -> impl IntoView {
    let params = use_params_map();
    let gid = move || params.read().get("id").unwrap_or_default();
    let navigate = use_navigate();

    let detail = RwSignal::new(None::<GalleryDetail>);
    let title = RwSignal::new(String::new());
    let slug = RwSignal::new(String::new());

    let load = {
        let navigate = navigate.clone();
        move || {
            let id = gid();
            if id.is_empty() {
                return;
            }
            let navigate = navigate.clone();
            spawn_local(async move {
                match api::gallery_detail(&id).await {
                    Ok(d) => {
                        title.set(d.gallery.title.clone());
                        slug.set(d.gallery.slug.clone());
                        detail.set(Some(d));
                    }
                    Err(ApiError::Unauthorized) => {
                        navigate("/login", Default::default())
                    }
                    Err(_) => {}
                }
            });
        }
    };
    Effect::new({
        let load = load.clone();
        move |_| load()
    });

    let published = move || detail.get().map(|d| d.gallery.published).unwrap_or(false);

    let save_meta = {
        let load = load.clone();
        move |publish: Option<bool>| {
            let id = gid();
            let body = UpdateGallery {
                title: Some(title.get()),
                slug: Some(slug.get()),
                published: publish,
                position: None,
            };
            let load = load.clone();
            spawn_local(async move {
                let _ = api::update_gallery(&id, &body).await;
                load();
            });
        }
    };

    let confirm_del = RwSignal::new(false);
    let delete_gallery = {
        let navigate = navigate.clone();
        move |_| {
            let id = gid();
            let navigate = navigate.clone();
            spawn_local(async move {
                if api::delete_gallery(&id).await.is_ok() {
                    navigate("/", Default::default());
                }
            });
        }
    };

    let photos = move || detail.get().map(|d| d.photos).unwrap_or_default();

    let reorder = {
        let load = load.clone();
        move |from: usize, to: usize| {
            let mut ps = photos();
            if to >= ps.len() {
                return;
            }
            ps.swap(from, to);
            let ids = ps.iter().map(|p| p.id).collect();
            let id = gid();
            let load = load.clone();
            spawn_local(async move {
                let _ = api::reorder_photos(&id, &Reorder { ids }).await;
                load();
            });
        }
    };

    let upload = {
        let load = load.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let Some(t) = ev.target() else { return };
            let form: web_sys::HtmlFormElement = t.unchecked_into();
            let Ok(data) = web_sys::FormData::new_with_form(&form) else {
                return;
            };
            let load = load.clone();
            spawn_local(async move {
                if api::upload_photo(data).await.is_ok() {
                    form.reset();
                    load();
                }
            });
        }
    };

    view! {
        <div class="mx-auto max-w-5xl space-y-6 p-6">
            <a href="/" class="text-sm/6 text-zinc-500">"← Portafolio"</a>

            <Card>
                <div class="space-y-3">
                    <Heading text="Editar galería" />
                    <Field label="Título">
                        <input class=INPUT prop:value=title
                            on:input=move |e| title.set(event_target_value(&e)) />
                    </Field>
                    <Field label="Slug">
                        <input class=INPUT prop:value=slug
                            on:input=move |e| slug.set(event_target_value(&e)) />
                    </Field>
                    <div class="flex gap-3">
                        <Button on:click={
                            let s = save_meta.clone();
                            move |_| s(None)
                        }>"Guardar"</Button>
                        <Button kind="outline" on:click={
                            let s = save_meta.clone();
                            move |_| s(Some(!published()))
                        }>
                            {move || if published() { "Despublicar" } else { "Publicar" }}
                        </Button>
                        <span class="grow"></span>
                        {move || if confirm_del.get() {
                            view! {
                                <Button kind="plain" on:click=delete_gallery.clone()>
                                    "Confirmar borrado"
                                </Button>
                            }.into_any()
                        } else {
                            view! {
                                <Button kind="plain"
                                    on:click=move |_| confirm_del.set(true)>
                                    "Borrar galería"
                                </Button>
                            }.into_any()
                        }}
                    </div>
                </div>
            </Card>

            <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                {move || photos().into_iter().enumerate().map(|(i, p)| {
                    let pid = p.id.to_string();
                    let alt = RwSignal::new(p.alt.clone());
                    let pid_alt = pid.clone();
                    let pid_del = pid.clone();
                    let load_a = load.clone();
                    let load_d = load.clone();
                    let reorder_u = reorder.clone();
                    let reorder_d = reorder.clone();
                    let total = photos().len();
                    view! {
                        <div class="rounded-xl border border-zinc-950/10 bg-white p-3 shadow-sm space-y-2">
                            <img src=thumb_src(&p) alt=p.alt.clone()
                                class="aspect-4/3 w-full rounded-lg object-cover bg-zinc-100" />
                            <input class=INPUT prop:value=alt
                                on:input=move |e| alt.set(event_target_value(&e)) />
                            <div class="flex gap-2 text-sm">
                                <button class="text-zinc-500 disabled:opacity-30"
                                    disabled={i == 0}
                                    on:click=move |_| reorder_u(i, i.wrapping_sub(1))>"↑"</button>
                                <button class="text-zinc-500 disabled:opacity-30"
                                    disabled={i + 1 >= total}
                                    on:click=move |_| reorder_d(i, i + 1)>"↓"</button>
                                <span class="grow"></span>
                                <button class="text-zinc-500"
                                    on:click=move |_| {
                                        let id = pid_alt.clone();
                                        let body = UpdatePhoto { alt: Some(alt.get()), position: None };
                                        let load = load_a.clone();
                                        spawn_local(async move {
                                            let _ = api::update_photo(&id, &body).await;
                                            load();
                                        });
                                    }>"Guardar alt"</button>
                                <button class="text-red-600"
                                    on:click=move |_| {
                                        let id = pid_del.clone();
                                        let load = load_d.clone();
                                        spawn_local(async move {
                                            let _ = api::delete_photo(&id).await;
                                            load();
                                        });
                                    }>"Borrar"</button>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>

            <Card>
                <h2 class="mb-3 text-sm/6 font-semibold text-zinc-950">"Subir foto"</h2>
                <form on:submit=upload class="space-y-3">
                    <input type="hidden" name="gallery_id" prop:value=gid />
                    <Field label="Texto alternativo">
                        <input class=INPUT name="alt" />
                    </Field>
                    <Field label="Archivo">
                        <input class=INPUT type="file" name="file" accept="image/*" />
                    </Field>
                    <Button>"Subir"</Button>
                </form>
            </Card>
        </div>
    }
}
