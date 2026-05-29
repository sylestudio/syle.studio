//! Inner views for non-text blocks (code, divider, image). The drag handle and
//! delete affordance live in the `BlockRow` wrapper; these render only content.

use leptos::prelude::*;
use syle_types::Block;

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
    let (id_src, id_alt) = (id.clone(), id);
    let has_src = !src.is_empty();
    let shown = src.clone();
    view! {
        <div class="rounded-lg border border-white/10 p-2">
            {has_src.then(|| view! {
                <img src=shown alt=alt.clone() class="mx-auto max-h-80 rounded" />
            })}
            // src commits on blur/Enter so the preview <img> doesn't re-mount per keystroke.
            <input
                class="mt-1 w-full bg-transparent text-xs text-zinc-400 outline-none \
                    placeholder:text-zinc-600"
                placeholder="URL de la imagen (p. ej. /media/…)"
                prop:value=src
                on:change=move |e| {
                    let val = event_target_value(&e);
                    blocks.update(|v| {
                        if let Some(Block::Image { src, .. }) =
                            v.iter_mut().find(|b| b.id() == id_src)
                        {
                            *src = val.clone();
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
