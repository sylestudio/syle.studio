//! Inline formatting toolbar. Applies marks to the current selection via
//! `execCommand` (the browser handles caret/Range), then re-serializes the
//! focused block from the DOM so the model stays the source of record.

use super::dom;
use leptos::prelude::*;
use syle_types::Block;

/// After a formatting command, sync the focused block's model from its DOM.
fn sync(blocks: RwSignal<Vec<Block>>) {
    let Some(el) = dom::active_editable() else { return };
    let Some(id) = el.get_attribute("data-block") else { return };
    let spans = dom::serialize_inline(&el);
    blocks.update(|v| {
        if let Some(b) = v.iter_mut().find(|b| b.id() == id) {
            super::content::set_spans(b, spans);
        }
    });
}

#[component]
pub fn FormatToolbar(blocks: RwSignal<Vec<Block>>) -> impl IntoView {
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
        </div>
    }
}
