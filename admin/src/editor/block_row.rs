//! A single editable block row. Dispatches on the block variant; text blocks
//! share one contenteditable whose model syncs FROM the DOM (caret-safe). The
//! drag handle and delete affordance are wrapped around the inner view so the
//! caret-stable subtree stays untouched. Structural edits live in `ops`.

use super::content::{self, Kind};
use super::ops::{self, Blocks, Dragging, Slash, SlashQuery};
use super::slash::SlashMenu;
use super::{chrome, dom, views};
use leptos::html;
use leptos::prelude::*;
use syle_render::render_inline;
use syle_types::Block;
use wasm_bindgen::JsCast;
use web_sys::Element;

#[component]
pub fn BlockRow(
    block: Block,
    blocks: Blocks,
    slash: Slash,
    slash_query: SlashQuery,
    dragging: Dragging,
) -> impl IntoView {
    let id = block.id().to_string();

    // The verified contenteditable subtree is built unchanged; drag/delete
    // chrome is wrapped around it additively so the caret path is untouched.
    let inner = match &block {
        Block::Code { language, code, .. } => {
            views::code_view(blocks, id.clone(), language.clone(), code.clone()).into_any()
        }
        Block::Divider { .. } => views::divider_view().into_any(),
        Block::Image { src, alt, .. } => {
            views::image_view(blocks, id.clone(), src.clone(), alt.clone()).into_any()
        }
        _ => text_view(block, blocks, slash, slash_query, id.clone()).into_any(),
    };

    let (drag_id, drop_id, del_id) = (id.clone(), id.clone(), id);
    view! {
        <div
            class="group relative flex items-start gap-1"
            on:dragover=move |e: leptos::ev::DragEvent| e.prevent_default()
            on:drop=move |e: leptos::ev::DragEvent| {
                e.prevent_default();
                ops::reorder(blocks, dragging, &drop_id);
            }
        >
            <span
                class="mt-1.5 shrink-0 cursor-grab select-none px-0.5 text-zinc-600 opacity-0 \
                    transition-opacity hover:text-zinc-400 group-hover:opacity-100"
                draggable="true"
                title="Arrastra para reordenar"
                on:dragstart=move |e: leptos::ev::DragEvent| {
                    if let Some(dt) = e.data_transfer() {
                        let _ = dt.set_data("text/plain", &drag_id);
                    }
                    dragging.set(Some(drag_id.clone()));
                }
                on:dragend=move |_| dragging.set(None)
            >
                "⠿"
            </span>
            <div class="min-w-0 flex-1">{inner}</div>
            {delete_btn(blocks, del_id)}
        </div>
    }
}

fn text_view(
    block: Block,
    blocks: Blocks,
    slash: Slash,
    slash_query: SlashQuery,
    id: String,
) -> impl IntoView {
    let kind = content::kind_of(&block);
    let seed = render_inline(&content::spans_of(&block));
    let node = NodeRef::<html::Div>::new();

    // Seed the DOM once on mount; the model never writes back in while typing.
    Effect::new(move |_| {
        if let Some(div) = node.get() {
            dom::set_html(div.unchecked_ref::<Element>(), &seed);
        }
    });

    let on_input = {
        let id = id.clone();
        move |_| {
            if let Some(div) = node.get() {
                ops::handle_input(blocks, slash, slash_query, &id, div.unchecked_ref::<Element>());
            }
        }
    };
    let on_keydown = {
        let id = id.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            let Some(div) = node.get() else { return };
            let el = div.unchecked_ref::<Element>();
            let slash_here = slash.get_untracked().as_deref() == Some(id.as_str());
            match ev.key().as_str() {
                // While the slash menu is open, Enter picks the top match and
                // Esc closes it — otherwise Enter would split the block.
                "Enter" if slash_here => {
                    ev.prevent_default();
                    match content::filter_kinds(&slash_query.get_untracked()).first().copied() {
                        Some(k) => ops::apply_convert(blocks, slash, id.clone(), k),
                        None => slash.set(None),
                    }
                }
                "Escape" if slash_here => {
                    ev.prevent_default();
                    slash.set(None);
                    dom::focus_block(&id, false);
                }
                // Tab / Shift+Tab nest and un-nest list items in place.
                "Tab" if matches!(kind, Kind::Bullet | Kind::Numbered | Kind::Todo) => {
                    ev.prevent_default();
                    ops::indent_block(blocks, &id, !ev.shift_key());
                }
                "Enter" if !ev.shift_key() => {
                    ev.prevent_default();
                    ops::handle_enter(blocks, slash, &id, kind, el);
                }
                "Backspace" => {
                    if ops::handle_backspace(blocks, &id, el) {
                        ev.prevent_default();
                    }
                }
                _ => {}
            }
        }
    };

    let todo_checked = matches!(&block, Block::Todo { checked: true, .. });
    let callout_emoji = match &block {
        Block::Callout { emoji, .. } => emoji.clone(),
        _ => String::new(),
    };
    let marker = chrome::marker_view(kind, todo_checked, callout_emoji, blocks, id.clone());

    let row_class = match kind {
        Kind::Quote => "flex gap-2 border-l-2 border-white/25 pl-3 py-0.5",
        Kind::Callout => {
            "flex gap-2 rounded-lg border border-amber-400/20 bg-amber-400/5 px-3 py-2"
        }
        Kind::Bullet | Kind::Numbered | Kind::Todo => "flex items-start gap-2 py-0.5",
        _ => "py-0.5",
    };

    let id_attr = id.clone();
    let ph_id = id.clone();
    let editable = view! {
        <div
            contenteditable="true"
            node_ref=node
            data-block=id_attr
            data-ph=chrome::placeholder_text(kind)
            on:input=on_input
            on:keydown=on_keydown
            // Reactive only on the class attr (never innerHTML) → caret-safe.
            class=move || {
                let base = chrome::editable_class(kind);
                let empty = blocks.with(|v| {
                    v.iter()
                        .find(|b| b.id() == ph_id)
                        .map(content::is_empty_text)
                        .unwrap_or(false)
                });
                if empty {
                    format!("{base} is-empty")
                } else {
                    base.to_string()
                }
            }
        ></div>
    };

    let mg_id = id.clone();
    let indent_margin = move || {
        let n = blocks.with(|v| {
            v.iter()
                .find(|b| b.id() == mg_id)
                .map(content::indent_of)
                .unwrap_or(0)
        });
        format!("{}rem", f32::from(n) * 1.25)
    };

    view! {
        <div
            class="group relative rounded-lg px-1 hover:bg-white/[0.02]"
            style:margin-left=indent_margin
        >
            <div class=row_class>
                {marker}
                {editable}
            </div>
            {move || (slash.get().as_deref() == Some(id.as_str())).then(|| {
                let bid = id.clone();
                let bid2 = id.clone();
                view! {
                    <SlashMenu
                        query=slash_query
                        on_pick=Callback::new(move |k| ops::apply_convert(blocks, slash, bid.clone(), k))
                        on_close=Callback::new(move |_| {
                            slash.set(None);
                            dom::focus_block(&bid2, false);
                        })
                    />
                }
            })}
        </div>
    }
}

fn delete_btn(blocks: Blocks, id: String) -> impl IntoView {
    view! {
        <button
            class="absolute right-1 top-1 hidden rounded p-1 text-xs text-zinc-500 \
                hover:bg-white/10 hover:text-white group-hover:block"
            title="Eliminar bloque"
            on:click=move |_| ops::remove_block(blocks, &id)
        >
            "✕"
        </button>
    }
}
