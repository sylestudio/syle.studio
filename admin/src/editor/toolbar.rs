//! Inline formatting toolbar. Marks are applied to the current selection via
//! `execCommand` (the browser handles caret/Range), then the focused block is
//! re-serialized from the DOM so the model stays the source of record. Links
//! use a saved-selection + `surroundContents` path (see `dom::create_link`)
//! with a small URL field, since they need a target the toolbar must collect.

use super::{content, dom};
use leptos::html;
use leptos::prelude::*;
use syle_render::coerce_href;
use syle_types::Block;

type Blocks = RwSignal<Vec<Block>>;

/// After a formatting command, sync the focused block's model from its DOM.
fn sync(blocks: Blocks) {
    let Some(el) = dom::active_editable() else {
        return;
    };
    let Some(id) = el.get_attribute("data-block") else {
        return;
    };
    let spans = dom::serialize_inline(&el);
    blocks.update(|v| {
        if let Some(b) = v.iter_mut().find(|b| b.id() == id) {
            content::set_spans(b, spans);
        }
    });
}

/// Wrap the saved selection in a normalized link, then sync the block it lived
/// in (the contenteditable is no longer active, so we serialize it by id).
fn apply_link(
    blocks: Blocks,
    open: RwSignal<bool>,
    url: RwSignal<String>,
    block: RwSignal<Option<String>>,
) {
    if let Some(href) = coerce_href(&url.get_untracked()) {
        if dom::create_link(&href) {
            if let Some(id) = block.get_untracked() {
                if let Some(spans) = dom::serialize_block(&id) {
                    blocks.update(|v| {
                        if let Some(b) = v.iter_mut().find(|b| b.id() == id) {
                            content::set_spans(b, spans);
                        }
                    });
                }
                dom::focus_block(&id, false);
            }
        }
    }
    open.set(false);
    url.set(String::new());
}

#[component]
pub fn FormatToolbar(blocks: Blocks) -> impl IntoView {
    let link_open = RwSignal::new(false);
    let link_url = RwSignal::new(String::new());
    let link_block = RwSignal::new(None::<String>);
    let url_ref = NodeRef::<html::Input>::new();

    // Focus the URL field whenever the link editor opens.
    Effect::new(move |_| {
        if link_open.get() {
            if let Some(input) = url_ref.get() {
                let _ = input.focus();
            }
        }
    });

    // `mousedown` + preventDefault keeps the contenteditable selection alive.
    let cmd = move |c: &'static str| {
        move |e: leptos::ev::MouseEvent| {
            e.prevent_default();
            dom::exec(c);
            sync(blocks);
        }
    };
    let code = move |e: leptos::ev::MouseEvent| {
        e.prevent_default();
        dom::wrap_inline_code();
        sync(blocks);
    };
    let open_link = move |e: leptos::ev::MouseEvent| {
        e.prevent_default();
        let id = dom::active_editable().and_then(|el| el.get_attribute("data-block"));
        // Save the live selection before focus moves to the URL field.
        if dom::save_selection() && id.is_some() {
            link_block.set(id);
            link_url.set(String::new());
            link_open.set(true);
        }
    };

    let btn = "grid h-8 w-8 place-items-center rounded-lg text-sm text-zinc-300 \
        hover:bg-white/10 hover:text-white transition-colors";
    view! {
        <div class="sticky top-0 z-10 flex items-center gap-1 rounded-xl border \
            border-white/10 bg-zinc-900/80 p-1 backdrop-blur">
            <button class=btn title="Negrita (⌘B)" on:mousedown=cmd("bold")>
                <span class="font-bold">"B"</span>
            </button>
            <button class=btn title="Cursiva (⌘I)" on:mousedown=cmd("italic")>
                <span class="italic">"I"</span>
            </button>
            <button class=btn title="Tachado" on:mousedown=cmd("strikeThrough")>
                <span class="line-through">"S"</span>
            </button>
            <button class=btn title="Código" on:mousedown=code>
                <span class="font-mono text-xs">"</>"</span>
            </button>
            <span class="mx-0.5 h-5 w-px bg-white/10"></span>
            <button class=btn title="Enlace" on:mousedown=open_link>
                <span class="text-base leading-none">"🔗"</span>
            </button>
            {move || link_open.get().then(|| view! {
                <div class="ml-1 flex items-center gap-1">
                    <input
                        node_ref=url_ref
                        class="h-8 w-56 rounded-lg border border-white/10 bg-white/5 px-2 \
                            text-sm text-zinc-100 outline-none placeholder:text-zinc-500 \
                            focus:border-white/25"
                        placeholder="https://… o /ruta"
                        prop:value=move || link_url.get()
                        on:input=move |e| link_url.set(event_target_value(&e))
                        on:keydown=move |e: leptos::ev::KeyboardEvent| match e.key().as_str() {
                            "Enter" => {
                                e.prevent_default();
                                apply_link(blocks, link_open, link_url, link_block);
                            }
                            "Escape" => {
                                e.prevent_default();
                                link_open.set(false);
                                link_url.set(String::new());
                            }
                            _ => {}
                        }
                    />
                    <button
                        class="h-8 rounded-lg bg-white/10 px-2.5 text-sm text-white \
                            hover:bg-white/20"
                        on:mousedown=move |e| {
                            e.prevent_default();
                            apply_link(blocks, link_open, link_url, link_block);
                        }
                    >
                        "Enlazar"
                    </button>
                    <button
                        class=btn
                        title="Cancelar"
                        on:mousedown=move |e| {
                            e.prevent_default();
                            link_open.set(false);
                            link_url.set(String::new());
                        }
                    >
                        "✕"
                    </button>
                </div>
            })}
        </div>
    }
}
