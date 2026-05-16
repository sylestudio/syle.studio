use crate::api::{self, ApiError};
use crate::ui::{Button, Card, Field, Heading, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use syle_types::{BlogPost, Gallery, NewGallery, NewPost, PostStatus};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

type Galleries = RwSignal<Vec<Gallery>>;
type Posts = RwSignal<Vec<BlogPost>>;

fn reload(galleries: Galleries, posts: Posts, auth_failed: RwSignal<bool>) {
    spawn_local(async move {
        if api::me().await == Err(ApiError::Unauthorized) {
            auth_failed.set(true);
            return;
        }
        if let Ok(v) = api::list_galleries().await {
            galleries.set(v);
        }
        if let Ok(v) = api::list_posts().await {
            posts.set(v);
        }
    });
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let galleries: Galleries = RwSignal::new(Vec::new());
    let posts: Posts = RwSignal::new(Vec::new());
    let auth_failed = RwSignal::new(false);
    // Obtained in the component body so the Router context is in scope; the
    // returned closure is cloned into effects/handlers.
    let navigate = use_navigate();

    Effect::new(move |_| reload(galleries, posts, auth_failed));
    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if auth_failed.get() {
                navigate("/login", Default::default());
            }
        }
    });

    let g_slug = RwSignal::new(String::new());
    let g_title = RwSignal::new(String::new());
    let create_gallery = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let n = NewGallery {
            slug: g_slug.get(),
            title: g_title.get(),
            position: 0,
            published: false,
        };
        spawn_local(async move {
            if api::create_gallery(&n).await.is_ok() {
                g_slug.set(String::new());
                g_title.set(String::new());
                reload(galleries, posts, auth_failed);
            }
        });
    };

    let p_slug = RwSignal::new(String::new());
    let p_title = RwSignal::new(String::new());
    let p_body = RwSignal::new(String::new());
    let create_post = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let n = NewPost {
            slug: p_slug.get(),
            title: p_title.get(),
            body_md: p_body.get(),
            status: PostStatus::Draft,
        };
        spawn_local(async move {
            if api::create_post(&n).await.is_ok() {
                p_slug.set(String::new());
                p_title.set(String::new());
                p_body.set(String::new());
                reload(galleries, posts, auth_failed);
            }
        });
    };

    let logout = move |_| {
        let navigate = navigate.clone();
        spawn_local(async move {
            api::logout().await;
            navigate("/login", Default::default());
        });
    };

    view! {
        <div class="mx-auto max-w-5xl space-y-8 p-6">
            <div class="flex items-center justify-between">
                <Heading text="Portafolio" />
                <Button kind="plain" on:click=logout>"Salir"</Button>
            </div>

            <div class="grid gap-6 lg:grid-cols-2">
                <Card>
                    <h2 class="mb-4 text-sm/6 font-semibold text-zinc-950">"Galerías"</h2>
                    <ul class="mb-4 divide-y divide-zinc-950/5">
                        {move || galleries.get().into_iter().map(|g| view! {
                            <li class="flex justify-between py-2 text-sm/6">
                                <span class="text-zinc-950">{g.title}</span>
                                <span class="text-zinc-500">
                                    {if g.published { "publicada" } else { "borrador" }}
                                </span>
                            </li>
                        }).collect_view()}
                    </ul>
                    <form on:submit=create_gallery class="space-y-3">
                        <Field label="Slug">
                            <input class=INPUT prop:value=g_slug
                                on:input=move |e| g_slug.set(event_target_value(&e)) />
                        </Field>
                        <Field label="Título">
                            <input class=INPUT prop:value=g_title
                                on:input=move |e| g_title.set(event_target_value(&e)) />
                        </Field>
                        <Button>"Crear galería"</Button>
                    </form>
                </Card>

                <Card>
                    <h2 class="mb-4 text-sm/6 font-semibold text-zinc-950">"Blog"</h2>
                    <ul class="mb-4 divide-y divide-zinc-950/5">
                        {move || posts.get().into_iter().map(|p| view! {
                            <li class="flex justify-between py-2 text-sm/6">
                                <span class="text-zinc-950">{p.title}</span>
                                <span class="text-zinc-500">
                                    {match p.status {
                                        PostStatus::Published => "publicado",
                                        PostStatus::Draft => "borrador",
                                    }}
                                </span>
                            </li>
                        }).collect_view()}
                    </ul>
                    <form on:submit=create_post class="space-y-3">
                        <Field label="Slug">
                            <input class=INPUT prop:value=p_slug
                                on:input=move |e| p_slug.set(event_target_value(&e)) />
                        </Field>
                        <Field label="Título">
                            <input class=INPUT prop:value=p_title
                                on:input=move |e| p_title.set(event_target_value(&e)) />
                        </Field>
                        <Field label="Contenido (Markdown)">
                            <textarea class=INPUT rows="4" prop:value=p_body
                                on:input=move |e| p_body.set(event_target_value(&e)) />
                        </Field>
                        <Button>"Crear borrador"</Button>
                    </form>
                </Card>
            </div>

            <PhotoUpload galleries=galleries />
        </div>
    }
}

#[component]
fn PhotoUpload(galleries: Galleries) -> impl IntoView {
    let status = RwSignal::new(String::new());

    let upload = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(target) = ev.target() else { return };
        let form: web_sys::HtmlFormElement = target.unchecked_into();
        let Ok(data) = web_sys::FormData::new_with_form(&form) else {
            return;
        };
        status.set("Subiendo…".into());
        spawn_local(async move {
            match api::upload_photo(data).await {
                Ok(_) => status.set("Foto subida".into()),
                Err(_) => status.set("Error al subir".into()),
            }
        });
    };

    view! {
        <Card>
            <h2 class="mb-4 text-sm/6 font-semibold text-zinc-950">"Subir foto"</h2>
            <form on:submit=upload class="space-y-3">
                <Field label="Galería">
                    <select name="gallery_id" class=INPUT>
                        {move || galleries.get().into_iter().map(|g| view! {
                            <option value=g.id.to_string()>{g.title}</option>
                        }).collect_view()}
                    </select>
                </Field>
                <Field label="Texto alternativo">
                    <input class=INPUT name="alt" />
                </Field>
                <Field label="Archivo">
                    <input class=INPUT type="file" name="file" accept="image/*" />
                </Field>
                <Button>"Subir"</Button>
                <p class="text-sm/6 text-zinc-500">{move || status.get()}</p>
            </form>
        </Card>
    }
}
