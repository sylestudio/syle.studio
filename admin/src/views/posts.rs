use crate::api::{self, ApiError};
use crate::ui::{use_toaster, Badge, Button, Field, Heading, SlideOver, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use syle_types::{is_valid_slug, BlogPost, NewPost, PostStatus};
use wasm_bindgen_futures::spawn_local;

type Posts = RwSignal<Vec<BlogPost>>;

fn reload(posts: Posts, auth_failed: RwSignal<bool>) {
    spawn_local(async move {
        match api::list_posts().await {
            Ok(items) => posts.set(items),
            Err(ApiError::Unauthorized) => auth_failed.set(true),
            Err(_) => {}
        }
    });
}

#[component]
pub fn PostsView() -> impl IntoView {
    let posts: Posts = RwSignal::new(Vec::new());
    let auth_failed = RwSignal::new(false);
    let goto = RwSignal::new(None::<String>);
    let navigate = use_navigate();

    Effect::new(move |_| reload(posts, auth_failed));
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
        let post = NewPost {
            slug: slug.get().trim().into(),
            title: title.get().trim().into(),
            blocks: Vec::new(),
            status: PostStatus::Draft,
        };
        saving.set(true);
        spawn_local(async move {
            match api::create_post(&post).await {
                Ok(post) => {
                    toast.ok("Borrador creado");
                    open.set(false);
                    goto.set(Some(format!("/posts/{}", post.id)));
                }
                Err(_) => toast.err("No se pudo crear el borrador"),
            }
            saving.set(false);
        });
    };

    view! {
        <div class="space-y-8">
            <div class="flex flex-wrap items-end justify-between gap-4">
                <div class="space-y-1">
                    <Heading text="Blog" />
                    <p class="text-sm/6 text-zinc-400">
                        "Entradas publicadas y borradores editoriales."
                    </p>
                </div>
                <Button on:click=move |_| open.set(true)>"Nueva entrada"</Button>
            </div>

            {move || {
                let items = posts.get();
                if items.is_empty() {
                    view! {
                        <p class="text-sm/6 text-zinc-500">"Sin entradas todavía."</p>
                    }.into_any()
                } else {
                    view! {
                        <div class="rounded-2xl border border-white/10 bg-white/4 p-1.5">
                            <ul class="bezel-core divide-y divide-white/5 overflow-hidden \
                                rounded-xl bg-zinc-900">
                                {items.into_iter().map(|post| {
                                    let published = post.status == PostStatus::Published;
                                    view! {
                                        <li class="reveal-item">
                                            <a
                                                href=format!("/posts/{}", post.id)
                                                class="group flex items-center justify-between gap-3 px-5 \
                                                    py-4 transition-colors hover:bg-white/3"
                                            >
                                                <span class="truncate text-sm/6 font-medium \
                                                    text-white transition-colors \
                                                    group-hover:text-zinc-300">
                                                    {post.title}
                                                </span>
                                                {if published {
                                                    view! {
                                                        <Badge tone="green">"publicado"</Badge>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <Badge tone="amber">"borrador"</Badge>
                                                    }.into_any()
                                                }}
                                            </a>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        </div>
                    }.into_any()
                }
            }}

            <SlideOver open=open title="Nueva entrada">
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
                        "Se abrirá el editor con vista previa para escribir el contenido."
                    </p>
                    <Button disabled=Signal::derive(move || !valid.get() || saving.get())>
                        {move || if saving.get() { "Creando…" } else { "Crear borrador" }}
                    </Button>
                </form>
            </SlideOver>
        </div>
    }
}
