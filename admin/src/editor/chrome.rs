//! Per-row visual chrome: the leading marker (bullet / number / checkbox /
//! callout emoji), the editable's typographic class, and the empty-block
//! placeholder text. No caret logic lives here — these only style the row.

use super::content::Kind;
use leptos::prelude::*;
use syle_types::Block;

type Blocks = RwSignal<Vec<Block>>;

/// Typographic class for a text block's contenteditable surface. `relative` so
/// the empty-block placeholder `::before` anchors to the editable's own start
/// (after any list marker), not the row wrapper.
pub(super) fn editable_class(kind: Kind) -> &'static str {
    match kind {
        Kind::H1 => "relative flex-1 text-2xl font-semibold text-white outline-none",
        Kind::H2 => "relative flex-1 text-xl font-semibold text-white outline-none",
        Kind::H3 => "relative flex-1 text-lg font-semibold text-white outline-none",
        Kind::Quote => "relative flex-1 italic text-zinc-300 outline-none",
        _ => "relative flex-1 text-sm/6 text-zinc-200 outline-none",
    }
}

/// Ghost hint shown (via the `is-empty` class + `data-ph`, see main.css) when a
/// block is focused and empty. Per-kind so headings read "Título", etc.
pub(super) fn placeholder_text(kind: Kind) -> &'static str {
    match kind {
        Kind::H1 => "Título 1",
        Kind::H2 => "Título 2",
        Kind::H3 => "Título 3",
        Kind::Quote => "Cita",
        Kind::Callout => "Escribe un llamado…",
        Kind::Bullet | Kind::Numbered => "Elemento de lista",
        Kind::Todo => "Tarea",
        _ => "Escribe, o pulsa “/” para comandos",
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
