//! Per-row visual chrome: the leading marker (bullet / number / checkbox /
//! callout emoji), the editable's typographic class, and the empty-block
//! placeholder text. No caret logic lives here — these only style the row.

use super::content::Kind;
use leptos::prelude::*;
use syle_types::Block;

type Blocks = RwSignal<Vec<Block>>;

/// Typographic class for a text block's contenteditable surface.
pub(super) fn editable_class(kind: Kind) -> &'static str {
    match kind {
        Kind::H1 => "flex-1 text-2xl font-semibold text-white outline-none",
        Kind::H2 => "flex-1 text-xl font-semibold text-white outline-none",
        Kind::H3 => "flex-1 text-lg font-semibold text-white outline-none",
        Kind::Quote => "flex-1 italic text-zinc-300 outline-none",
        _ => "flex-1 text-sm/6 text-zinc-200 outline-none",
    }
}

/// Leading marker for list / callout rows; empty for plain text and headings.
pub(super) fn marker_view(
    kind: Kind,
    checked: bool,
    emoji: String,
    blocks: Blocks,
    id: String,
) -> impl IntoView {
    match kind {
        Kind::Bullet => {
            view! { <span class="mt-1.5 select-none text-zinc-500">"•"</span> }.into_any()
        }
        Kind::Numbered => {
            view! { <span class="mt-0.5 select-none font-mono text-xs text-zinc-500">"1."</span> }
                .into_any()
        }
        Kind::Todo => view! {
            <input
                type="checkbox"
                prop:checked=checked
                class="mt-1.5 accent-amber-400"
                on:change=move |_| {
                    blocks.update(|v| {
                        if let Some(Block::Todo { checked, .. }) =
                            v.iter_mut().find(|b| b.id() == id)
                        {
                            *checked = !*checked;
                        }
                    });
                }
            />
        }
        .into_any(),
        Kind::Callout => view! { <span class="select-none">{emoji}</span> }.into_any(),
        _ => ().into_any(),
    }
}
