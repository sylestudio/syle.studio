use crate::api::{self, ApiError};
use crate::ui::{use_toaster, Badge, Button, Field, Heading, SlideOver, Tile, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use syle_types::{is_valid_project_url, NewProject, Project};
use wasm_bindgen_futures::spawn_local;

type Projects = RwSignal<Vec<Project>>;

fn reload(projects: Projects, auth_failed: RwSignal<bool>) {
    spawn_local(async move {
        match api::list_projects().await {
            Ok(items) => projects.set(items),
            Err(ApiError::Unauthorized) => auth_failed.set(true),
            Err(_) => {}
        }
    });
}

#[component]
fn ProjectCard(project: Project) -> impl IntoView {
    let href = format!("/projects/{}", project.id);
    let cover = project.cover.as_ref().map(|image| image.src.clone());
    let title = project.title;
    let url = project.url;
    let published = project.published;

    view! {
        <a href=href class="group reveal-item block">
            <Tile>
                <div class="relative aspect-4/3 bg-zinc-800">
                    {if let Some(src) = cover {
                        view! {
                            <img src=src alt=""
                                class="absolute inset-0 block h-full w-full object-cover \
                                    transition duration-500 ease-fluid group-hover:scale-105 \
                                    motion-reduce:transition-none motion-reduce:group-hover:scale-100" />
                        }.into_any()
                    } else {
                        view! {
                            <div class="flex h-full items-center justify-center text-xs/5 text-zinc-600">
                                "Sin portada"
                            </div>
                        }.into_any()
                    }}
                </div>
                <div class="space-y-1 px-4 py-3">
                    <div class="flex items-center justify-between gap-3">
                        <span class="truncate text-sm/6 font-medium text-white \
                            transition-colors group-hover:text-zinc-300">
                            {title}
                        </span>
                        {if published {
                            view! { <Badge tone="green">"publicado"</Badge> }.into_any()
                        } else {
                            view! { <Badge>"borrador"</Badge> }.into_any()
                        }}
                    </div>
                    <p class="truncate text-xs/5 text-zinc-500">{url}</p>
                </div>
            </Tile>
        </a>
    }
}

#[component]
pub fn ProjectsView() -> impl IntoView {
    let projects: Projects = RwSignal::new(Vec::new());
    let auth_failed = RwSignal::new(false);
    let goto = RwSignal::new(None::<String>);
    let navigate = use_navigate();

    Effect::new(move |_| reload(projects, auth_failed));
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
    let title = RwSignal::new(String::new());
    let url = RwSignal::new(String::new());
    let category = RwSignal::new(String::new());
    let position = RwSignal::new("0".to_string());
    let saving = RwSignal::new(false);
    let toast = use_toaster();
    let valid = Signal::derive(move || {
        !title.get().trim().is_empty()
            && is_valid_project_url(url.get().trim())
            && position.get().trim().parse::<i32>().is_ok()
    });

    let create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if !valid.get() || saving.get() {
            toast.err("Revisa el título, la URL y la posición");
            return;
        }
        let project = NewProject {
            title: title.get().trim().to_string(),
            url: url.get().trim().to_string(),
            category: category.get().trim().to_string(),
            position: position.get().trim().parse().unwrap_or_default(),
            published: false,
            cover: None,
        };
        saving.set(true);
        spawn_local(async move {
            match api::create_project(&project).await {
                Ok(project) => {
                    toast.ok("Proyecto creado");
                    open.set(false);
                    goto.set(Some(format!("/projects/{}", project.id)));
                }
                Err(_) => toast.err("No se pudo crear el proyecto"),
            }
            saving.set(false);
        });
    };

    view! {
        <div class="space-y-8">
            <div class="flex flex-wrap items-end justify-between gap-4">
                <div class="space-y-1">
                    <Heading text="Proyectos" />
                    <p class="text-sm/6 text-zinc-400">
                        "Enlaces independientes que aparecen junto a las galerías."
                    </p>
                </div>
                <Button on:click=move |_| open.set(true)>"Nuevo proyecto"</Button>
            </div>

            {move || {
                let items = projects.get();
                if items.is_empty() {
                    view! {
                        <p class="text-sm/6 text-zinc-500">
                            "Aún no hay proyectos independientes."
                        </p>
                    }.into_any()
                } else {
                    view! {
                        <div class="grid grid-cols-2 gap-5 sm:grid-cols-3 lg:grid-cols-4">
                            {items.into_iter().map(|project| view! {
                                <ProjectCard project=project />
                            }).collect_view()}
                        </div>
                    }.into_any()
                }
            }}

            <SlideOver open=open title="Nuevo proyecto">
                <form on:submit=create class="space-y-4">
                    <Field label="Título">
                        <input class=INPUT prop:value=title
                            on:input=move |e| title.set(event_target_value(&e)) />
                    </Field>
                    <Field label="URL de destino">
                        <input class=INPUT prop:value=url placeholder="/proyectos/dango"
                            on:input=move |e| url.set(event_target_value(&e)) />
                        <p class="text-xs/5 text-zinc-500">
                            "Ruta interna con / o URL completa http(s)."
                        </p>
                    </Field>
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
                    <p class="text-xs/5 text-zinc-500">
                        "Después podrás subir la portada y publicarlo."
                    </p>
                    <Button disabled=Signal::derive(move || !valid.get() || saving.get())>
                        {move || if saving.get() { "Creando…" } else { "Crear proyecto" }}
                    </Button>
                </form>
            </SlideOver>
        </div>
    }
}
