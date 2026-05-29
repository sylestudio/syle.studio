//! Slash (`/`) command menu: pick a block type to convert the current block.
//! The list refilters live against the query typed after `/`; Enter/Esc are
//! handled by the block's keydown (see `block_row`), this only renders.

use super::content::{self, Kind};
use leptos::prelude::*;

#[component]
pub fn SlashMenu(
    query: RwSignal<String>,
    on_pick: Callback<Kind>,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="absolute z-30 mt-1 w-64 overflow-hidden rounded-xl border border-white/10 \
            bg-zinc-800 p-1 shadow-2xl shadow-black/40">
            <div class="px-2.5 pb-1 pt-1.5 text-[0.6875rem] font-medium uppercase tracking-wide \
                text-zinc-500">
                "Convertir en"
            </div>
            {move || {
                let kinds = content::filter_kinds(&query.get());
                if kinds.is_empty() {
                    view! {
                        <div class="px-2.5 py-2 text-sm text-zinc-500">"Sin resultados"</div>
                    }
                    .into_any()
                } else {
                    kinds
                        .into_iter()
                        .map(|kind| {
                            view! {
                                <button
                                    class="flex w-full items-center gap-3 rounded-lg px-2.5 py-1.5 \
                                        text-left text-sm text-zinc-200 hover:bg-white/10"
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
                        })
                        .collect_view()
                        .into_any()
                }
            }}
            <div class="mt-0.5 flex items-center justify-between border-t border-white/5 \
                px-2.5 py-1 text-[0.6875rem] text-zinc-500">
                <span>"↵ elegir · esc cerrar"</span>
                <button
                    class="rounded px-1 hover:text-zinc-300"
                    on:mousedown=move |e| {
                        e.prevent_default();
                        on_close.run(());
                    }
                >
                    "Cerrar"
                </button>
            </div>
        </div>
    }
}
