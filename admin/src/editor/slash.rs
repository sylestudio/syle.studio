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
                                        {marker(kind)}
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

/// Slash-menu marker for a block kind. Headings keep their compact `H1`/`H2`/`H3`
/// text (the universal editor convention); every other kind renders a monochrome
/// Material Icons glyph — same family/viewBox as the link icon in `toolbar.rs` —
/// inheriting the chip's `currentColor`.
fn marker(kind: Kind) -> AnyView {
    // Material Icons (baseline) path data, fetched verbatim — kept one-per-line
    // intact (a backslash line-continuation would silently drop path segments).
    let path = match kind {
        Kind::H1 => return view! { "H1" }.into_any(),
        Kind::H2 => return view! { "H2" }.into_any(),
        Kind::H3 => return view! { "H3" }.into_any(),
        Kind::Paragraph => "M14 17H4v2h10v-2zm6-8H4v2h16V9zM4 15h16v-2H4v2zM4 5v2h16V5H4z",
        Kind::Bullet => "M4 10.5c-.83 0-1.5.67-1.5 1.5s.67 1.5 1.5 1.5s1.5-.67 1.5-1.5s-.67-1.5-1.5-1.5zm0-6c-.83 0-1.5.67-1.5 1.5S3.17 7.5 4 7.5S5.5 6.83 5.5 6S4.83 4.5 4 4.5zm0 12c-.83 0-1.5.68-1.5 1.5s.68 1.5 1.5 1.5s1.5-.68 1.5-1.5s-.67-1.5-1.5-1.5zM7 19h14v-2H7v2zm0-6h14v-2H7v2zm0-8v2h14V5H7z",
        Kind::Numbered => "M2 17h2v.5H3v1h1v.5H2v1h3v-4H2v1zm1-9h1V4H2v1h1v3zm-1 3h1.8L2 13.1v.9h3v-1H3.2L5 10.9V10H2v1zm5-6v2h14V5H7zm0 14h14v-2H7v2zm0-6h14v-2H7v2z",
        Kind::Todo => "M19 5v14H5V5h14m0-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z",
        Kind::Quote => "M6 17h3l2-4V7H5v6h3zm8 0h3l2-4V7h-6v6h3z",
        Kind::Code => "M9.4 16.6L4.8 12l4.6-4.6L8 6l-6 6l6 6l1.4-1.4zm5.2 0l4.6-4.6l-4.6-4.6L16 6l6 6l-6 6l-1.4-1.4z",
        Kind::Divider => "M4 11h16v2H4z",
        Kind::Callout => "M9 21c0 .5.4 1 1 1h4c.6 0 1-.5 1-1v-1H9v1zm3-19C8.1 2 5 5.1 5 9c0 2.4 1.2 4.5 3 5.7V17c0 .5.4 1 1 1h6c.6 0 1-.5 1-1v-2.3c1.8-1.3 3-3.4 3-5.7c0-3.9-3.1-7-7-7z",
        Kind::Image => "M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z",
    };
    view! {
        <svg viewBox="0 0 24 24" fill="currentColor" class="h-4 w-4" aria-hidden="true">
            <path d=path></path>
        </svg>
    }
    .into_any()
}
