//! Inner views for non-text blocks (code, divider, image). The drag handle and
//! delete affordance live in the `BlockRow` wrapper; these render only content.

use crate::api;
use crate::ui::use_toaster;
use leptos::prelude::*;
use syle_types::Block;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{FormData, HtmlInputElement};

type Blocks = RwSignal<Vec<Block>>;

pub fn code_view(blocks: Blocks, id: String, language: String, code: String) -> impl IntoView {
    let (id_lang, id_code) = (id.clone(), id);
    view! {
        <div class="rounded-lg border border-white/10 bg-black/30 p-2">
            <input
                class="mb-1 w-40 bg-transparent font-mono text-xs text-zinc-400 outline-none \
                    placeholder:text-zinc-600"
                placeholder="lenguaje"
                prop:value=language
                on:input=move |e| {
                    let val = event_target_value(&e);
                    blocks.update(|v| {
                        if let Some(Block::Code { language, .. }) =
                            v.iter_mut().find(|b| b.id() == id_lang)
                        {
                            *language = val.clone();
                        }
                    });
                }
            />
            <textarea
                class="block w-full resize-y bg-transparent font-mono text-sm text-zinc-200 \
                    outline-none"
                rows="3"
                prop:value=code
                on:input=move |e| {
                    let val = event_target_value(&e);
                    blocks.update(|v| {
                        if let Some(Block::Code { code, .. }) =
                            v.iter_mut().find(|b| b.id() == id_code)
                        {
                            *code = val.clone();
                        }
                    });
                }
            ></textarea>
        </div>
    }
}

pub fn divider_view() -> impl IntoView {
    view! { <hr class="my-2 border-white/15" /> }
}

pub fn image_view(blocks: Blocks, id: String, src: String, alt: String) -> impl IntoView {
    let id_up = id.clone();
    let (id_src, id_alt) = (id.clone(), id);
    let toast = use_toaster();
    let has_src = !src.is_empty();
    let shown = src.clone();
    view! {
        <div class="rounded-lg border border-white/10 p-2">
            {has_src.then(|| view! {
                <img src=shown alt=alt.clone() class="mx-auto max-h-80 rounded" />
            })}
            // Upload a file (ingested server-side) or paste a URL — either sets src.
            <label class="mt-1 block cursor-pointer text-xs text-zinc-400 hover:text-white">
                "Subir imagen…"
                <input
                    type="file"
                    accept="image/jpeg,image/png,image/webp"
                    class="hidden"
                    on:change=move |e| {
                        let Some(input) =
                            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                        else {
                            return;
                        };
                        let Some(file) = input.files().and_then(|f| f.get(0)) else {
                            return;
                        };
                        let Ok(form) = FormData::new() else { return };
                        if form.append_with_blob("file", &file).is_err() {
                            return;
                        }
                        let id = id_up.clone();
                        spawn_local(async move {
                            match api::upload_blog_asset(form).await {
                                Ok(img) => {
                                    blocks.update(|v| {
                                        if let Some(Block::Image {
                                            src,
                                            variants,
                                            placeholder,
                                            width,
                                            height,
                                            ..
                                        }) = v.iter_mut().find(|b| b.id() == id)
                                        {
                                            *src = img.src;
                                            *variants = img.variants;
                                            *placeholder = img.placeholder;
                                            *width = img.width;
                                            *height = img.height;
                                        }
                                    });
                                }
                                // The pipeline rejects anything it can't decode
                                // (or that's too large) with a 400 — tell the user
                                // instead of failing silently.
                                Err(api::ApiError::Status(400)) => {
                                    toast.err("Formato no admitido o imagen dañada. Usa JPG, PNG o WebP.")
                                }
                                Err(_) => toast.err("No se pudo subir la imagen"),
                            }
                        });
                    }
                />
            </label>
            // src commits on blur/Enter so the preview <img> doesn't re-mount per keystroke.
            <input
                class="mt-1 w-full bg-transparent text-xs text-zinc-400 outline-none \
                    placeholder:text-zinc-600"
                placeholder="URL de la imagen (p. ej. /media/…)"
                prop:value=src
                on:change=move |e| {
                    let val = event_target_value(&e);
                    blocks.update(|v| {
                        if let Some(Block::Image {
                            src,
                            variants,
                            placeholder,
                            width,
                            height,
                            ..
                        }) = v.iter_mut().find(|b| b.id() == id_src)
                        {
                            // A hand-typed URL has no renditions of its own;
                            // clear any from a prior upload so it renders as a
                            // plain <img>, not a stale <picture>.
                            *src = val.clone();
                            variants.clear();
                            placeholder.clear();
                            *width = 0;
                            *height = 0;
                        }
                    });
                }
            />
            <input
                class="mt-1 w-full bg-transparent text-center text-xs text-zinc-500 outline-none \
                    placeholder:text-zinc-600"
                placeholder="texto alternativo"
                prop:value=alt
                on:input=move |e| {
                    let val = event_target_value(&e);
                    blocks.update(|v| {
                        if let Some(Block::Image { alt, .. }) =
                            v.iter_mut().find(|b| b.id() == id_alt)
                        {
                            *alt = val.clone();
                        }
                    });
                }
            />
        </div>
    }
}
