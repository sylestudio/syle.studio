use crate::api::{self, ApiError};
use crate::editor::BlockEditor;
use crate::ui::{use_toaster, Badge, Button, Card, Field, Modal, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use syle_render::render_blocks;
use syle_types::{is_valid_slug, Block, PostStatus, UpdatePost};
use wasm_bindgen_futures::spawn_local;

/// Readable prose styling for the dark live preview (mirrors the public site
/// structurally; both render the same block HTML via `syle-render`).
const PROSE: &str = "min-h-[60vh] overflow-y-auto rounded-xl border border-white/10 \
    bg-white/5 p-5 text-sm/6 text-zinc-300 \
    [&_h1]:mt-0 [&_h1]:mb-2 [&_h1]:text-xl [&_h1]:font-semibold [&_h1]:text-white \
    [&_h2]:mt-4 [&_h2]:mb-2 [&_h2]:text-lg [&_h2]:font-semibold [&_h2]:text-white \
    [&_h3]:mt-3 [&_h3]:font-semibold [&_h3]:text-white \
    [&_p]:my-2 [&_strong]:text-white [&_em]:italic [&_a]:text-blue-400 [&_a]:underline \
    [&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5 \
    [&_code]:rounded [&_code]:bg-white/10 [&_code]:px-1 [&_code]:py-0.5 [&_code]:text-zinc-200 \
    [&_pre]:my-3 [&_pre]:rounded-lg [&_pre]:bg-black/40 [&_pre]:p-3 [&_pre]:text-xs \
    [&_pre_code]:bg-transparent [&_pre_code]:p-0 \
    [&_.hl-kw]:text-[#c792ea] [&_.hl-str]:text-[#c3e88d] [&_.hl-num]:text-[#f78c6c] \
    [&_.hl-com]:text-[#8b8678] [&_.hl-com]:italic \
    [&_blockquote]:border-l-2 [&_blockquote]:border-white/20 [&_blockquote]:pl-3 [&_blockquote]:text-zinc-400 \
    [&_hr]:my-4 [&_hr]:border-white/15 [&_img]:rounded-lg \
    [&_.todo-list]:list-none [&_.todo-list]:pl-1 \
    [&_.callout]:flex [&_.callout]:gap-2 [&_.callout]:rounded-lg [&_.callout]:border \
    [&_.callout]:border-amber-400/20 [&_.callout]:bg-amber-400/5 [&_.callout]:p-3";

#[component]
pub fn PostEditor() -> impl IntoView {
    let params = use_params_map();
    let pid = move || params.read().get("id").unwrap_or_default();
    let navigate = use_navigate();

    let title = RwSignal::new(String::new());
    let slug = RwSignal::new(String::new());
    let blocks = RwSignal::new(Vec::<Block>::new());
    let published = RwSignal::new(false);
    let loaded = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let toast = use_toaster();

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
                        blocks.set(if p.blocks.is_empty() {
                            vec![crate::editor::empty_paragraph()]
                        } else {
                            p.blocks
                        });
                        published.set(p.status == PostStatus::Published);
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

    let save = {
        let load = load.clone();
        move || {
            let id = pid();
            let s = slug.get().trim().to_string();
            if !is_valid_slug(&s) {
                toast.err("Slug inválido: usa minúsculas, números y guiones");
                return;
            }
            let req = UpdatePost {
                slug: Some(s),
                title: Some(title.get()),
                blocks: Some(blocks.get()),
                status: Some(if published.get() {
                    PostStatus::Published
                } else {
                    PostStatus::Draft
                }),
            };
            if saving.get() {
                return;
            }
            saving.set(true);
            let load = load.clone();
            spawn_local(async move {
                match api::update_post(&id, &req).await {
                    Ok(_) => {
                        toast.ok("Entrada guardada");
                        load();
                    }
                    Err(_) => toast.err("Error al guardar"),
                }
                saving.set(false);
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
                } else {
                    toast.err("No se pudo borrar");
                }
            });
        }
    };

    let preview = move || render_blocks(&blocks.get());
    let slug_ok = Signal::derive(move || is_valid_slug(slug.get().trim()));
    let save_btn = save.clone();
    let toggle_pub = {
        let save = save.clone();
        move |_| {
            published.update(|p| *p = !*p);
            save();
        }
    };

    view! {
        <div class="space-y-8">
            <div class="sticky top-0 max-lg:top-14 z-20 -mx-6 -mt-6 flex flex-wrap items-center gap-4 \
                border-b border-white/10 bg-zinc-900/80 px-6 py-4 backdrop-blur \
                lg:-mx-10 lg:-mt-10 lg:px-10">
                <a href="/" class="rounded-lg p-1.5 text-zinc-400 hover:bg-white/10 \
                    hover:text-white transition-colors" aria-label="Volver">"←"</a>
                <input
                    class="min-w-40 flex-1 bg-transparent text-xl font-semibold \
                        tracking-tight text-white outline-none placeholder:text-zinc-600"
                    prop:value=title
                    placeholder="Título de la entrada"
                    on:input=move |e| title.set(event_target_value(&e))
                />
                {move || if published.get() {
                    view! { <Badge tone="green">"publicado"</Badge> }.into_any()
                } else {
                    view! { <Badge tone="amber">"borrador"</Badge> }.into_any()
                }}
                <div class="flex items-center gap-2">
                    <Button disabled=Signal::derive(move || saving.get() || !slug_ok.get())
                        on:click=move |_| save_btn()>
                        {move || if saving.get() { "Guardando…" } else { "Guardar" }}
                    </Button>
                    <Button kind="outline" on:click=toggle_pub>
                        {move || if published.get() { "Despublicar" } else { "Publicar" }}
                    </Button>
                    <Button kind="plain" on:click=move |_| confirm_del.set(true)>
                        "Borrar"
                    </Button>
                </div>
            </div>

            {move || (!loaded.get()).then(|| view! {
                <p class="text-sm/6 text-zinc-500">"Cargando…"</p>
            })}

            <div class="grid gap-6 lg:grid-cols-2">
                <div class="space-y-1.5">
                    <p class="text-sm/6 font-medium text-zinc-300">"Editor"</p>
                    <BlockEditor blocks=blocks />
                </div>
                <div class="space-y-1.5">
                    <p class="text-sm/6 font-medium text-zinc-300">"Vista previa"</p>
                    <div class=PROSE inner_html=preview></div>
                </div>
            </div>

            <Card>
                <Field label="Slug">
                    <input class=INPUT prop:value=slug
                        on:input=move |e| slug.set(event_target_value(&e)) />
                    {move || (!slug_ok.get()).then(|| view! {
                        <p class="text-xs/5 text-red-400">
                            "Solo minúsculas, números y guiones (sin espacios ni puntos)."
                        </p>
                    })}
                </Field>
            </Card>

            <Modal open=confirm_del title="Eliminar entrada">
                <p class="text-sm/6 text-zinc-400">
                    "Esta acción no se puede deshacer."
                </p>
                <div class="mt-5 flex justify-end gap-2">
                    <Button kind="plain" on:click=move |_| confirm_del.set(false)>
                        "Cancelar"
                    </Button>
                    <Button kind="danger" on:click=delete.clone()>
                        "Eliminar"
                    </Button>
                </div>
            </Modal>
        </div>
    }
}
