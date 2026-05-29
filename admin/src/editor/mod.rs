//! Block editor for the CRM — a Notion-style editor built entirely in
//! Leptos/WASM. The document is a `RwSignal<Vec<Block>>`; while a block is
//! focused the contenteditable DOM is the source of truth and the model syncs
//! FROM it on input (caret-stable). Structural edits (convert/split/merge/
//! reorder) are plain `Vec<Block>` operations.

mod block_row;
mod chrome;
mod content;
mod dom;
mod ops;
mod slash;
mod toolbar;
mod views;

use block_row::BlockRow;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use syle_types::{Block, History};
use toolbar::FormatToolbar;

pub use content::empty_paragraph;

/// Idle delay (ms) before a typing burst is coalesced into one undo step.
const CHECKPOINT_DEBOUNCE_MS: u32 = 450;

#[component]
pub fn BlockEditor(blocks: RwSignal<Vec<Block>>) -> impl IntoView {
    let slash = RwSignal::new(None::<String>);
    let slash_query = RwSignal::new(String::new());
    let dragging = RwSignal::new(None::<String>);

    // Document-level undo/redo. The signal is the live source of truth; this
    // mirrors settled states into a history stack. `generation` is folded into
    // the row key so a restore remounts every row (text, code and image alike)
    // from the model — the only uniform way to refresh the non-reactive inputs.
    let history = StoredValue::new(History::new(blocks.get_untracked()));
    let generation = RwSignal::new(0u64);
    // `Timeout` is !Send, so its handle lives in thread-local storage.
    let pending = StoredValue::new_local(None::<Timeout>);

    // Coalesce changes: each edit (re)arms a timer; it fires once typing idles
    // and records the current document (a no-op if unchanged, so undo→record
    // can't clobber redo). Structural edits ride the same path.
    Effect::new(move |_| {
        blocks.track();
        let t = Timeout::new(CHECKPOINT_DEBOUNCE_MS, move || {
            history.update_value(|h| h.record(blocks.get_untracked()));
        });
        pending.set_value(Some(t));
    });

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

    // Ctrl/⌘+Z undoes; Ctrl/⌘+Shift+Z or Ctrl/⌘+Y redoes. Caught on the
    // container so it works from any focused block, pre-empting the browser's
    // per-field native undo.
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if !(ev.ctrl_key() || ev.meta_key()) {
            return;
        }
        let restored = match ev.key().to_lowercase().as_str() {
            "z" if !ev.shift_key() => {
                ev.prevent_default();
                // Flush the in-progress typing as one step, then step back.
                history
                    .try_update_value(|h| {
                        h.record(blocks.get_untracked());
                        h.undo().cloned()
                    })
                    .flatten()
            }
            "z" | "y" => {
                ev.prevent_default();
                // Redo must not record first — that would discard the redo stack.
                history.try_update_value(|h| h.redo().cloned()).flatten()
            }
            _ => None,
        };
        if let Some(state) = restored {
            // Keep the caret near where it was so chained undos stay in-editor.
            let target = dom::active_editable()
                .and_then(|el| el.get_attribute("data-block"))
                .filter(|id| state.iter().any(|b| b.id() == id))
                .or_else(|| state.first().map(|b| b.id().to_string()));
            pending.set_value(None);
            blocks.set(state);
            generation.update(|g| *g += 1);
            if let Some(id) = target {
                dom::after_render(move || dom::focus_block(&id, false));
            }
        }
    };

    view! {
        <div class="space-y-3" on:keydown=on_keydown>
            <FormatToolbar blocks=blocks />
            <div class="min-h-[40vh] space-y-0.5">
                <For
                    each=move || blocks.get()
                    key=move |b| (content::signature(b), generation.get_untracked())
                    let:block
                >
                    <BlockRow
                        block=block
                        blocks=blocks
                        slash=slash
                        slash_query=slash_query
                        dragging=dragging
                    />
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
