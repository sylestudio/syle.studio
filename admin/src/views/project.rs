use crate::api::{self, ApiError};
use crate::ui::{use_toaster, Badge, Button, Card, Field, Modal, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate, use_params_map};
use syle_types::{is_valid_project_url, Project, UpdateProject};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn ProjectView() -> impl IntoView {
    let params = use_params_map();
    // The route mounts a fresh editor for each project. Event handlers only
    // need the current id; an untracked read avoids re-running the load effect
    // while the component is being torn down after delete/navigation.
    let pid = move || params.read_untracked().get("id").unwrap_or_default();
    let pathname = use_location().pathname;
    let navigate = use_navigate();

    let project = RwSignal::new(None::<Project>);
    let title = RwSignal::new(String::new());
    let url = RwSignal::new(String::new());
    let category = RwSignal::new(String::new());
    let position = RwSignal::new("0".to_string());
    let loaded = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let uploading = RwSignal::new(false);
    let deleting = RwSignal::new(false);
    let toast = use_toaster();

    let load = {
        let navigate = navigate.clone();
        move || {
            if deleting.get_untracked() {
                return;
            }
            let id = pid();
            if id.is_empty() || pathname.get_untracked() != format!("/projects/{id}") {
                return;
            }
            let navigate = navigate.clone();
            spawn_local(async move {
                match api::get_project(&id).await {
                    Ok(value) => {
                        title.set(value.title.clone());
                        url.set(value.url.clone());
                        category.set(value.category.clone());
                        position.set(value.position.to_string());
                        project.set(Some(value));
                        loaded.set(true);
                    }
                    Err(ApiError::Unauthorized) => navigate("/login", Default::default()),
                    Err(_) => loaded.set(true),
                }
            });
        }
    };
    Effect::new({
        let load = load.clone();
        move |_| load()
    });

    let published = move || project.get().is_some_and(|value| value.published);
    let form_ok = Signal::derive(move || {
        !title.get().trim().is_empty()
            && is_valid_project_url(url.get().trim())
            && position.get().trim().parse::<i32>().is_ok()
    });

    let save_meta = {
        let load = load.clone();
        move |publish: Option<bool>| {
            if !form_ok.get() {
                toast.err("Revisa el título, la URL y la posición");
                return;
            }
            if saving.get() {
                return;
            }
            let id = pid();
            let body = UpdateProject {
                title: Some(title.get().trim().to_string()),
                url: Some(url.get().trim().to_string()),
                category: Some(category.get().trim().to_string()),
                position: position.get().trim().parse::<i32>().ok(),
                published: publish,
                cover: None,
                clear_cover: false,
            };
            saving.set(true);
            let load = load.clone();
            spawn_local(async move {
                match api::update_project(&id, &body).await {
                    Ok(_) => {
                        toast.ok("Proyecto guardado");
                        load();
                    }
                    Err(_) => toast.err("Error al guardar"),
                }
                saving.set(false);
            });
        }
    };

    let upload_cover = {
        let load = load.clone();
        move |ev: leptos::ev::Event| {
            let Some(input) = ev
                .target()
                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
            else {
                return;
            };
            let Some(file) = input.files().and_then(|files| files.get(0)) else {
                return;
            };
            input.set_value("");
            let Ok(form) = web_sys::FormData::new() else {
                return;
            };
            if form.append_with_blob("file", file.unchecked_ref()).is_err() {
                return;
            }

            let id = pid();
            uploading.set(true);
            let load = load.clone();
            spawn_local(async move {
                match api::upload_project_asset(form).await {
                    Ok(image) => {
                        let body = UpdateProject {
                            cover: Some(image),
                            ..Default::default()
                        };
                        match api::update_project(&id, &body).await {
                            Ok(_) => {
                                toast.ok("Portada actualizada");
                                load();
                            }
                            Err(_) => toast.err("No se pudo guardar la portada"),
                        }
                    }
                    Err(ApiError::Status(400)) => {
                        toast.err("Formato no admitido o imagen dañada. Usa JPG, PNG o WebP.")
                    }
                    Err(_) => toast.err("No se pudo subir la portada"),
                }
                uploading.set(false);
            });
        }
    };

    let clear_cover = {
        let load = load.clone();
        move |_| {
            let id = pid();
            let body = UpdateProject {
                clear_cover: true,
                ..Default::default()
            };
            let load = load.clone();
            spawn_local(async move {
                match api::update_project(&id, &body).await {
                    Ok(_) => {
                        toast.ok("Portada eliminada");
                        load();
                    }
                    Err(_) => toast.err("No se pudo eliminar la portada"),
                }
            });
        }
    };

    let confirm_delete = RwSignal::new(false);
    let delete_project = {
        let navigate = navigate.clone();
        move |_| {
            if deleting.get_untracked() {
                return;
            }
            deleting.set(true);
            let id = pid();
            let navigate = navigate.clone();
            spawn_local(async move {
                if api::delete_project(&id).await.is_ok() {
                    navigate("/projects", Default::default());
                } else {
                    deleting.set(false);
                    toast.err("No se pudo borrar el proyecto");
                }
            });
        }
    };

    view! {
        <div class="space-y-8">
            <div class="sticky top-0 max-lg:top-14 z-20 -mx-6 -mt-6 flex flex-wrap items-center gap-4 \
                border-b border-white/10 bg-zinc-900/80 px-6 py-4 backdrop-blur \
                lg:-mx-10 lg:-mt-10 lg:px-10">
                <a href="/projects" class="rounded-lg p-1.5 text-zinc-400 hover:bg-white/10 \
                    hover:text-white transition-colors" aria-label="Volver">"←"</a>
                <input
                    class="min-w-40 flex-1 bg-transparent text-xl font-semibold tracking-tight \
                        text-white outline-none placeholder:text-zinc-600"
                    prop:value=title
                    placeholder="Título del proyecto"
                    on:input=move |e| title.set(event_target_value(&e))
                />
                {move || if published() {
                    view! { <Badge tone="green">"publicado"</Badge> }.into_any()
                } else {
                    view! { <Badge>"borrador"</Badge> }.into_any()
                }}
                <div class="flex items-center gap-2">
                    <Button disabled=Signal::derive(move || saving.get() || !form_ok.get()) on:click={
                        let save = save_meta.clone();
                        move |_| save(None)
                    }>
                        {move || if saving.get() { "Guardando…" } else { "Guardar" }}
                    </Button>
                    <Button kind="outline" disabled=Signal::derive(move || !form_ok.get()) on:click={
                        let save = save_meta.clone();
                        move |_| save(Some(!published()))
                    }>
                        {move || if published() { "Despublicar" } else { "Publicar" }}
                    </Button>
                    <Button kind="plain" on:click=move |_| confirm_delete.set(true)>
                        "Borrar"
                    </Button>
                </div>
            </div>

            {move || (!loaded.get()).then(|| view! {
                <p class="text-sm/6 text-zinc-500">"Cargando…"</p>
            })}

            <div class="grid gap-6 lg:grid-cols-[minmax(0,1.35fr)_minmax(18rem,0.65fr)]">
                <Card>
                    <div class="space-y-4">
                        <h2 class="text-sm/6 font-semibold text-white">"Destino y orden"</h2>
                        <Field label="URL de destino">
                            <input class=INPUT prop:value=url placeholder="/proyectos/dango"
                                on:input=move |e| url.set(event_target_value(&e)) />
                            {move || (!is_valid_project_url(url.get().trim())).then(|| view! {
                                <p class="text-xs/5 text-red-400">
                                    "Usa una ruta que empiece con / o una URL http(s) completa."
                                </p>
                            })}
                        </Field>
                        <div class="grid gap-4 sm:grid-cols-2">
                            <Field label="Categoría / disciplina">
                                <input class=INPUT prop:value=category
                                    placeholder="Dirección · Prenda · Película"
                                    on:input=move |e| category.set(event_target_value(&e)) />
                            </Field>
                            <Field label="Posición en la grilla">
                                <input class=INPUT type="number" inputmode="numeric"
                                    prop:value=position
                                    on:input=move |e| position.set(event_target_value(&e)) />
                            </Field>
                        </div>
                        {move || is_valid_project_url(url.get().trim()).then(|| view! {
                            <a href=url.get() target="_blank" rel="noopener noreferrer"
                                class="inline-flex text-sm/6 text-zinc-400 underline \
                                    decoration-white/20 underline-offset-4 hover:text-white">
                                "Abrir destino ↗"
                            </a>
                        })}
                    </div>
                </Card>

                <Card>
                    <div class="space-y-4">
                        <h2 class="text-sm/6 font-semibold text-white">"Portada de la grilla"</h2>
                        {move || match project.get().and_then(|value| value.cover) {
                            Some(cover) => view! {
                                <img src=cover.src alt=""
                                    class="aspect-4/3 w-full rounded-xl object-cover" />
                            }.into_any(),
                            None => view! {
                                <div class="flex aspect-4/3 items-center justify-center rounded-xl \
                                    border border-dashed border-white/15 bg-white/2.5 \
                                    text-sm/6 text-zinc-500">
                                    "Sin portada"
                                </div>
                            }.into_any(),
                        }}
                        <div class="flex flex-wrap items-center gap-3">
                            <label class="cursor-pointer rounded-lg bg-white px-3 py-1.5 \
                                text-sm/6 font-semibold text-zinc-950 hover:bg-zinc-200">
                                {move || if uploading.get() { "Procesando…" } else { "Subir portada" }}
                                <input type="file" class="hidden"
                                    accept="image/jpeg,image/png,image/webp"
                                    disabled=move || uploading.get()
                                    on:change=upload_cover />
                            </label>
                            {move || project.get().and_then(|value| value.cover).is_some().then(|| view! {
                                <Button kind="plain" on:click=clear_cover.clone()>"Quitar"</Button>
                            })}
                        </div>
                        <p class="text-xs/5 text-zinc-500">
                            "El CRM genera AVIF y JPEG responsivos automáticamente."
                        </p>
                    </div>
                </Card>
            </div>

            <Modal open=confirm_delete title="Eliminar proyecto">
                <p class="text-sm/6 text-zinc-400">
                    "Se eliminará el enlace y su portada. La página de destino no se modifica."
                </p>
                <div class="mt-5 flex justify-end gap-2">
                    <Button kind="plain" on:click=move |_| confirm_delete.set(false)>
                        "Cancelar"
                    </Button>
                    <Button kind="danger" on:click=delete_project.clone()>"Eliminar"</Button>
                </div>
            </Modal>
        </div>
    }
}
