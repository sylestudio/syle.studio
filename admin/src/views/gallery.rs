use crate::api::{self, ApiError};
use crate::ui::{use_toaster, Button, Card, Field, Heading, INPUT};
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
    let loaded = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let confirm_photo = RwSignal::new(None::<String>);
    let uploading = RwSignal::new(String::new());
    let toast = use_toaster();

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
                        loaded.set(true);
                    }
                    Err(ApiError::Unauthorized) => {
                        navigate("/login", Default::default())
                    }
                    Err(_) => loaded.set(true),
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
            if saving.get() {
                return;
            }
            saving.set(true);
            let load = load.clone();
            spawn_local(async move {
                match api::update_gallery(&id, &body).await {
                    Ok(_) => {
                        toast.ok("Galería guardada");
                        load();
                    }
                    Err(_) => toast.err("Error al guardar"),
                }
                saving.set(false);
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
                } else {
                    toast.err("No se pudo borrar la galería");
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
            let input = |sel: &str| {
                form.query_selector(sel)
                    .ok()
                    .flatten()
                    .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
            };
            let id = gid();
            let alt = input("input[name=\"alt\"]")
                .map(|i| i.value())
                .unwrap_or_default();
            let Some(files) = input("input[name=\"file\"]").and_then(|i| i.files())
            else {
                return;
            };
            let n = files.length();
            if n == 0 {
                return;
            }
            let load = load.clone();
            spawn_local(async move {
                let mut ok = 0u32;
                for k in 0..n {
                    let Some(file) = files.get(k) else { continue };
                    let Ok(fd) = web_sys::FormData::new() else { continue };
                    let _ = fd.append_with_str("gallery_id", &id);
                    let _ = fd.append_with_str("alt", &alt);
                    let _ = fd.append_with_blob("file", file.unchecked_ref());
                    uploading.set(format!("Subiendo {}/{n}…", k + 1));
                    if api::upload_photo(fd).await.is_ok() {
                        ok += 1;
                    }
                }
                uploading.set(String::new());
                form.reset();
                if ok > 0 {
                    toast.ok(format!("{ok} foto(s) subidas"));
                }
                if ok < n {
                    toast.err("Algunas fotos fallaron");
                }
                load();
            });
        }
    };

    view! {
        <div class="mx-auto max-w-5xl space-y-6 p-6">
            <a href="/" class="text-sm/6 text-zinc-500">"← Portafolio"</a>
            {move || (!loaded.get()).then(|| view! {
                <p class="text-sm/6 text-zinc-500">"Cargando…"</p>
            })}

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
                        <Button disabled=Signal::derive(move || saving.get())
                            on:click={
                            let s = save_meta.clone();
                            move |_| s(None)
                        }>
                            {move || if saving.get() { "Guardando…" } else { "Guardar" }}
                        </Button>
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
                                            match api::update_photo(&id, &body).await {
                                                Ok(_) => { toast.ok("Alt guardado"); load(); }
                                                Err(_) => toast.err("Error al guardar alt"),
                                            }
                                        });
                                    }>"Guardar alt"</button>
                                {
                                    let pid_c = pid_del.clone();
                                    move || if confirm_photo.get().as_deref() == Some(pid_c.as_str()) {
                                        let id = pid_del.clone();
                                        let load = load_d.clone();
                                        view! {
                                            <button class="text-red-600 font-semibold"
                                                on:click=move |_| {
                                                    let id = id.clone();
                                                    let load = load.clone();
                                                    spawn_local(async move {
                                                        match api::delete_photo(&id).await {
                                                            Ok(_) => { toast.ok("Foto borrada"); load(); }
                                                            Err(_) => toast.err("No se pudo borrar"),
                                                        }
                                                    });
                                                }>"Confirmar"</button>
                                        }.into_any()
                                    } else {
                                        let pid_set = pid_c.clone();
                                        view! {
                                            <button class="text-red-600"
                                                on:click=move |_| confirm_photo.set(Some(pid_set.clone()))>
                                                "Borrar"
                                            </button>
                                        }.into_any()
                                    }
                                }
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>

            <Card>
                <h2 class="mb-3 text-sm/6 font-semibold text-zinc-950">"Subir foto"</h2>
                <form on:submit=upload class="space-y-3">
                    <Field label="Texto alternativo (se aplica a todas)">
                        <input class=INPUT name="alt" />
                    </Field>
                    <Field label="Archivos">
                        <input class=INPUT type="file" name="file"
                            accept="image/*" multiple />
                    </Field>
                    <div class="flex items-center gap-3">
                        <Button disabled=Signal::derive(move || !uploading.get().is_empty())>
                            "Subir"
                        </Button>
                        <span class="text-sm/6 text-zinc-500">
                            {move || uploading.get()}
                        </span>
                    </div>
                </form>
            </Card>
        </div>
    }
}
