use super::gallery_tile::PhotoTile;
use crate::api::{self, ApiError};
use crate::ui::{use_toaster, Badge, Button, Card, Field, Modal, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use syle_types::{is_valid_slug, GalleryDetail, Reorder, UpdateGallery};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn GalleryView() -> impl IntoView {
    let params = use_params_map();
    let gid = move || params.read().get("id").unwrap_or_default();
    let navigate = use_navigate();

    let detail = RwSignal::new(None::<GalleryDetail>);
    let title = RwSignal::new(String::new());
    let slug = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let notes = RwSignal::new(String::new());
    let category = RwSignal::new(String::new());
    let year = RwSignal::new(String::new());
    let loaded = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let confirm_photo = RwSignal::new(None::<String>);
    let confirm_photo_open = RwSignal::new(false);
    let uploading = RwSignal::new(String::new());
    let alt_sig = RwSignal::new(String::new());
    let editing = RwSignal::new(None::<web_sys::File>);
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
                        description.set(d.gallery.description.clone());
                        notes.set(d.gallery.notes.clone());
                        category.set(d.gallery.category.clone());
                        year.set(d.gallery.year.map(|y| y.to_string()).unwrap_or_default());
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
    let slug_ok = Signal::derive(move || is_valid_slug(slug.get().trim()));

    let save_meta = {
        let load = load.clone();
        move |publish: Option<bool>| {
            let id = gid();
            let s = slug.get().trim().to_string();
            if !is_valid_slug(&s) {
                toast.err("Slug inválido: usa minúsculas, números y guiones");
                return;
            }
            let body = UpdateGallery {
                title: Some(title.get()),
                slug: Some(s),
                published: publish,
                position: None,
                description: Some(description.get()),
                notes: Some(notes.get()),
                category: Some(category.get()),
                // Empty/invalid leaves the year unchanged (COALESCE merge).
                year: year.get().trim().parse::<i32>().ok(),
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
                    uploading.set(format!("Subiendo {}/{n} — 0%", k + 1));
                    let progress = move |frac: f64| {
                        // Bytes done (frac == 1.0) → the server is now decoding +
                        // encoding derivatives; show that phase instead of a bar
                        // frozen at 100%.
                        if frac >= 1.0 {
                            uploading.set(format!("Procesando {}/{n}… (codificando)", k + 1));
                        } else {
                            uploading.set(format!(
                                "Subiendo {}/{n} — {:.0}%",
                                k + 1,
                                frac * 100.0
                            ));
                        }
                    };
                    if api::upload_photo(fd, progress).await.is_ok() {
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

    // Editor path: bake one photo client-side, then push it through the same
    // `upload_photo` pipeline (with gallery_id + the shared alt) as a new photo.
    let on_baked = {
        let load = load.clone();
        Callback::new(move |blob: web_sys::Blob| {
            let id = gid();
            let alt = alt_sig.get_untracked();
            let Ok(fd) = web_sys::FormData::new() else { return };
            let _ = fd.append_with_str("gallery_id", &id);
            let _ = fd.append_with_str("alt", &alt);
            let _ = fd.append_with_blob_and_filename("file", &blob, "edit.png");
            editing.set(None);
            uploading.set("Procesando… (codificando)".to_string());
            let load = load.clone();
            spawn_local(async move {
                let progress = move |frac: f64| {
                    if frac >= 1.0 {
                        uploading.set("Procesando… (codificando)".to_string());
                    } else {
                        uploading.set(format!("Subiendo — {:.0}%", frac * 100.0));
                    }
                };
                match api::upload_photo(fd, progress).await {
                    Ok(_) => toast.ok("Foto subida"),
                    Err(_) => toast.err("No se pudo subir la foto"),
                }
                uploading.set(String::new());
                load();
            });
        })
    };

    view! {
        <div class="space-y-8">
            // Sticky contextual action bar
            <div class="sticky top-0 max-lg:top-14 z-20 -mx-6 -mt-6 flex flex-wrap items-center gap-4 \
                border-b border-white/10 bg-zinc-900/80 px-6 py-4 backdrop-blur \
                lg:-mx-10 lg:-mt-10 lg:px-10">
                <a href="/" class="rounded-lg p-1.5 text-zinc-400 hover:bg-white/10 \
                    hover:text-white transition-colors" aria-label="Volver">"←"</a>
                <input
                    class="min-w-40 flex-1 bg-transparent text-xl font-semibold \
                        tracking-tight text-white outline-none placeholder:text-zinc-600"
                    prop:value=title
                    placeholder="Título de la galería"
                    on:input=move |e| title.set(event_target_value(&e))
                />
                {move || if published() {
                    view! { <Badge tone="green">"publicada"</Badge> }.into_any()
                } else {
                    view! { <Badge>"borrador"</Badge> }.into_any()
                }}
                <div class="flex items-center gap-2">
                    <Button disabled=Signal::derive(move || saving.get() || !slug_ok.get()) on:click={
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
                    <Button kind="plain" on:click=move |_| confirm_del.set(true)>
                        "Borrar"
                    </Button>
                </div>
            </div>

            {move || (!loaded.get()).then(|| view! {
                <p class="text-sm/6 text-zinc-500">"Cargando…"</p>
            })}

            <Card>
                <div class="space-y-4">
                    <h2 class="text-sm/6 font-semibold text-white">
                        "Detalles del proyecto"
                    </h2>
                    <div class="grid gap-4 sm:grid-cols-2">
                        <Field label="Categoría / disciplina">
                            <input class=INPUT prop:value=category
                                placeholder="Dirección · Prenda · Película"
                                on:input=move |e| category.set(event_target_value(&e)) />
                        </Field>
                        <Field label="Año">
                            <input class=INPUT type="number" inputmode="numeric"
                                prop:value=year placeholder="2026"
                                on:input=move |e| year.set(event_target_value(&e)) />
                        </Field>
                    </div>
                    <Field label="Lede — intro del hero">
                        <textarea class=format!("{INPUT} min-h-24 leading-relaxed")
                            prop:value=description
                            on:input=move |e| description.set(event_target_value(&e)) />
                    </Field>
                    <Field label="Notas del proyecto — cierre">
                        <textarea class=format!("{INPUT} min-h-20 leading-relaxed")
                            prop:value=notes
                            on:input=move |e| notes.set(event_target_value(&e)) />
                    </Field>
                </div>
            </Card>

            <div class="grid gap-5 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                {
                    let load = load.clone();
                    let reorder = reorder.clone();
                    move || {
                        let total = photos().len();
                        photos().into_iter().enumerate().map({
                            let load = load.clone();
                            let reorder = reorder.clone();
                            move |(i, p)| {
                                let reload = Callback::new({
                                    let load = load.clone();
                                    move |_| load()
                                });
                                let reorder_cb = Callback::new({
                                    let reorder = reorder.clone();
                                    move |(f, t)| reorder(f, t)
                                });
                                view! {
                                    <PhotoTile
                                        photo=p index=i total=total
                                        confirm_photo=confirm_photo
                                        confirm_photo_open=confirm_photo_open
                                        reorder=reorder_cb reload=reload />
                                }
                            }
                        }).collect_view()
                    }
                }
            </div>

            <Card>
                <div class="space-y-4">
                    <div class="grid gap-4 sm:grid-cols-2">
                        <Field label="Slug">
                            <input class=INPUT prop:value=slug
                                on:input=move |e| slug.set(event_target_value(&e)) />
                            {move || (!slug_ok.get()).then(|| view! {
                                <p class="text-xs/5 text-red-400">
                                    "Solo minúsculas, números y guiones (sin espacios ni puntos)."
                                </p>
                            })}
                        </Field>
                    </div>
                    <div class="border-t border-white/10 pt-4">
                        <h2 class="mb-3 text-sm/6 font-semibold text-white">
                            "Subir fotos"
                        </h2>
                        <form on:submit=upload class="space-y-3">
                            <Field label="Texto alternativo (se aplica a todas)">
                                <input class=INPUT name="alt" prop:value=alt_sig
                                    on:input=move |e| alt_sig.set(event_target_value(&e)) />
                            </Field>
                            <Field label="Archivos">
                                <input class=INPUT type="file" name="file"
                                    accept="image/jpeg,image/png,image/webp" multiple />
                            </Field>
                            <div class="flex items-center gap-3">
                                <Button disabled=Signal::derive(move || !uploading.get().is_empty())>
                                    "Subir"
                                </Button>
                                <span class="text-sm/6 text-zinc-400">
                                    {move || uploading.get()}
                                </span>
                            </div>
                        </form>
                        <label class="mt-3 block cursor-pointer border-t border-white/10 pt-3 \
                            text-sm/6 text-zinc-300 hover:text-white">
                            "Editar una foto antes de subir…"
                            <input type="file" class="hidden"
                                accept="image/jpeg,image/png,image/webp"
                                on:change=move |e| {
                                    let Some(input) = e.target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    else { return; };
                                    let Some(file) = input.files().and_then(|f| f.get(0)) else {
                                        return;
                                    };
                                    input.set_value("");
                                    editing.set(Some(file));
                                }
                            />
                        </label>
                    </div>
                </div>
            </Card>

            <Modal open=confirm_del title="Eliminar galería">
                <p class="text-sm/6 text-zinc-400">
                    "Se eliminará la galería y todas sus fotos. Esta acción no se puede deshacer."
                </p>
                <div class="mt-5 flex justify-end gap-2">
                    <Button kind="plain" on:click=move |_| confirm_del.set(false)>
                        "Cancelar"
                    </Button>
                    <Button kind="danger" on:click=delete_gallery.clone()>
                        "Eliminar"
                    </Button>
                </div>
            </Modal>

            <Modal open=confirm_photo_open title="Eliminar foto">
                <p class="text-sm/6 text-zinc-400">
                    "Esta acción no se puede deshacer."
                </p>
                <div class="mt-5 flex justify-end gap-2">
                    <Button kind="plain" on:click=move |_| confirm_photo_open.set(false)>
                        "Cancelar"
                    </Button>
                    <Button kind="danger" on:click={
                        let load = load.clone();
                        move |_| {
                            let Some(id) = confirm_photo.get() else { return };
                            let load = load.clone();
                            confirm_photo_open.set(false);
                            spawn_local(async move {
                                match api::delete_photo(&id).await {
                                    Ok(_) => { toast.ok("Foto borrada"); load(); }
                                    Err(_) => toast.err("No se pudo borrar"),
                                }
                            });
                        }
                    }>
                        "Eliminar"
                    </Button>
                </div>
            </Modal>

            {move || editing.get().map(|file| view! {
                <crate::image_editor::ImageEditorModal
                    file=file
                    on_baked=on_baked
                    on_cancel=Callback::new(move |_| editing.set(None))
                />
            })}
        </div>
    }
}
