//! Block editor for the CRM — a Notion-style editor built entirely in
//! Leptos/WASM. The document is a `RwSignal<Vec<Block>>`; while a block is
//! focused the contenteditable DOM is the source of truth and the model syncs
//! FROM it on input (caret-stable). Structural edits (convert/split/merge/
//! reorder) are plain `Vec<Block>` operations.

mod block_row;
mod content;
mod dom;
mod slash;
mod toolbar;

use block_row::BlockRow;
use leptos::prelude::*;
use syle_types::Block;
use toolbar::FormatToolbar;

pub use content::empty_paragraph;

#[component]
pub fn BlockEditor(blocks: RwSignal<Vec<Block>>) -> impl IntoView {
    let slash = RwSignal::new(None::<String>);

    // Force tag-based formatting once, so the serializer sees <b>/<i> marks.
    Effect::new(move |_| dom::use_tag_formatting());

    let add_block = move |_| {
        let id = dom::new_id();
        let fid = id.clone();
        blocks.update(|v| {
            v.push(Block::Paragraph {
                id,
                content: Vec::new(),
            })
        });
        dom::after_render(move || dom::focus_block(&fid, true));
    };

    view! {
        <div class="space-y-3">
            <FormatToolbar blocks=blocks />
            <div class="min-h-[40vh] space-y-0.5">
                <For
                    each=move || blocks.get()
                    key=|b| content::signature(b)
                    let:block
                >
                    <BlockRow block=block blocks=blocks slash=slash />
                </For>
            </div>
            <button
                class="w-full rounded-lg border border-dashed border-white/10 py-2 text-sm \
                    text-zinc-500 hover:border-white/20 hover:text-zinc-300 transition-colors"
                on:click=add_block
            >
                "+ Agregar bloque"
            </button>
        </div>
    }
}
