//! Slash (`/`) command menu: pick a block type to convert the current block.

use super::content::Kind;
use leptos::prelude::*;

#[component]
pub fn SlashMenu(on_pick: Callback<Kind>, on_close: Callback<()>) -> impl IntoView {
    let items = Kind::menu().iter().copied().map(|kind| {
        view! {
            <button
                class="flex w-full items-center gap-3 rounded-lg px-2.5 py-1.5 text-left \
                    text-sm text-zinc-200 hover:bg-white/10"
                on:mousedown=move |e| {
                    e.prevent_default();
                    on_pick.run(kind);
                }
            >
                <span class="grid h-6 w-7 shrink-0 place-items-center rounded \
                    bg-white/5 font-mono text-xs text-zinc-400">
                    {kind.glyph()}
                </span>
                {kind.label()}
            </button>
        }
    }).collect_view();

    view! {
        <div class="absolute z-30 mt-1 w-60 overflow-hidden rounded-xl border border-white/10 \
            bg-zinc-800 p-1 shadow-2xl shadow-black/40">
            {items}
            <button
                class="mt-0.5 w-full rounded-lg px-2.5 py-1 text-left text-xs text-zinc-500 \
                    hover:bg-white/5"
                on:mousedown=move |e| {
                    e.prevent_default();
                    on_close.run(());
                }
            >
                "Cerrar"
            </button>
        </div>
    }
}
