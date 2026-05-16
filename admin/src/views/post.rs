use crate::api::{self, ApiError};
use crate::ui::{Button, Card, Field, Heading, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use syle_types::{PostStatus, UpdatePost};
use wasm_bindgen_futures::spawn_local;

fn md_to_html(src: &str) -> String {
    let parser = pulldown_cmark::Parser::new(src);
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, parser);
    out
}

#[component]
pub fn PostEditor() -> impl IntoView {
    let params = use_params_map();
    let pid = move || params.read().get("id").unwrap_or_default();
    let navigate = use_navigate();

    let title = RwSignal::new(String::new());
    let slug = RwSignal::new(String::new());
    let body = RwSignal::new(String::new());
    let published = RwSignal::new(false);
    let msg = RwSignal::new(String::new());

    let load = {
        let navigate = navigate.clone();
        move || {
            let id = pid();
            if id.is_empty() {
                return;
            }
            let navigate = navigate.clone();
            spawn_local(async move {
                match api::get_post(&id).await {
                    Ok(p) => {
                        title.set(p.title);
                        slug.set(p.slug);
                        body.set(p.body_md);
                        published.set(p.status == PostStatus::Published);
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

    let save = {
        let load = load.clone();
        move |_| {
            let id = pid();
            let req = UpdatePost {
                title: Some(title.get()),
                body_md: Some(body.get()),
                status: Some(if published.get() {
                    PostStatus::Published
                } else {
                    PostStatus::Draft
                }),
            };
            let load = load.clone();
            spawn_local(async move {
                match api::update_post(&id, &req).await {
                    Ok(_) => {
                        msg.set("Guardado".into());
                        load();
                    }
                    Err(_) => msg.set("Error al guardar".into()),
                }
            });
        }
    };

    let confirm_del = RwSignal::new(false);
    let delete = {
        let navigate = navigate.clone();
        move |_| {
            let id = pid();
            let navigate = navigate.clone();
            spawn_local(async move {
                if api::delete_post(&id).await.is_ok() {
                    navigate("/", Default::default());
                }
            });
        }
    };

    let preview = move || md_to_html(&body.get());

    view! {
        <div class="mx-auto max-w-5xl space-y-6 p-6">
            <a href="/" class="text-sm/6 text-zinc-500">"← Portafolio"</a>
            <Card>
                <div class="space-y-4">
                    <Heading text="Editar entrada" />
                    <Field label="Título">
                        <input class=INPUT prop:value=title
                            on:input=move |e| title.set(event_target_value(&e)) />
                    </Field>
                    <Field label="Slug">
                        <input class=INPUT prop:value=slug
                            on:input=move |e| slug.set(event_target_value(&e)) />
                    </Field>
                    <div class="grid gap-4 lg:grid-cols-2">
                        <Field label="Contenido (Markdown)">
                            <textarea class=format!("{INPUT} font-mono") rows="16"
                                prop:value=body
                                on:input=move |e| body.set(event_target_value(&e)) />
                        </Field>
                        <div>
                            <p class="mb-1.5 text-sm/6 font-medium text-zinc-950">
                                "Vista previa"
                            </p>
                            <div
                                class="min-h-80 rounded-lg border border-zinc-950/10 bg-white p-4 text-sm/6"
                                inner_html=preview
                            ></div>
                        </div>
                    </div>
                    <label class="flex items-center gap-2 text-sm/6 text-zinc-950">
                        <input type="checkbox" prop:checked=published
                            on:change=move |e| published.set(event_target_checked(&e)) />
                        "Publicado"
                    </label>
                    <div class="flex items-center gap-3">
                        <Button on:click=save.clone()>"Guardar"</Button>
                        <span class="grow"></span>
                        {move || if confirm_del.get() {
                            view! {
                                <Button kind="plain" on:click=delete.clone()>
                                    "Confirmar borrado"
                                </Button>
                            }.into_any()
                        } else {
                            view! {
                                <Button kind="plain"
                                    on:click=move |_| confirm_del.set(true)>
                                    "Borrar"
                                </Button>
                            }.into_any()
                        }}
                    </div>
                    <p class="text-sm/6 text-zinc-500">{move || msg.get()}</p>
                </div>
            </Card>
        </div>
    }
}
