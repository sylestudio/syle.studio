// LOC: one component that renders every block variant + its shared
// contenteditable handlers (input/shortcut/slash, Enter split, Backspace
// merge); splitting the per-variant view from the handlers would scatter one
// tightly-coupled unit across files. Kept lean.
//! A single editable block row. Dispatches on the block variant; text blocks
//! share one contenteditable whose model syncs FROM the DOM (caret-safe).

use super::content::{self, Kind};
use super::dom;
use super::slash::SlashMenu;
use leptos::html;
use leptos::prelude::*;
use syle_render::render_inline;
use syle_types::{Block, Span};
use wasm_bindgen::JsCast;
use web_sys::Element;

type Blocks = RwSignal<Vec<Block>>;
type Slash = RwSignal<Option<String>>;

fn set_block_spans(blocks: Blocks, id: &str, spans: Vec<Span>) {
    blocks.update(|v| {
        if let Some(b) = v.iter_mut().find(|b| b.id() == id) {
            content::set_spans(b, spans);
        }
    });
}

/// Re-seed the block's DOM from the model and place the caret, after render.
fn reseed_focus(blocks: Blocks, id: String, at_start: bool) {
    dom::after_render(move || {
        if let Some(html) = blocks
            .get_untracked()
            .iter()
            .find(|b| b.id() == id)
            .map(|b| render_inline(&content::spans_of(b)))
        {
            dom::reseed_block(&id, &html);
        }
        dom::focus_block(&id, at_start);
    });
}

/// Convert block `id` to `kind`, clearing its content. Dividers get a trailing
/// paragraph to land the caret. Closes the slash menu.
fn apply_convert(blocks: Blocks, slash: Slash, id: String, kind: Kind) {
    let para_id = dom::new_id();
    let is_divider = matches!(kind, Kind::Divider);
    let para_for_update = para_id.clone();
    blocks.update(|v| {
        if let Some(i) = v.iter().position(|b| b.id() == id) {
            let bid = v[i].id().to_string();
            v[i] = content::make(kind, bid, Vec::new());
            if is_divider {
                v.insert(
                    i + 1,
                    Block::Paragraph {
                        id: para_for_update.clone(),
                        content: Vec::new(),
                    },
                );
            }
        }
    });
    slash.set(None);
    reseed_focus(blocks, if is_divider { para_id } else { id }, true);
}

fn handle_input(blocks: Blocks, slash: Slash, id: &str, el: &Element) {
    // contenteditable renders a trailing space as a non-breaking space; fold it
    // back so Markdown triggers like "## " and "- " match.
    let text = el.text_content().unwrap_or_default().replace('\u{00A0}', " ");
    if let Some(kind) = content::detect_shortcut(&text) {
        apply_convert(blocks, slash, id.to_string(), kind);
        return;
    }
    if text == "/" {
        slash.set(Some(id.to_string()));
    } else if slash.get_untracked().as_deref() == Some(id) && !text.starts_with('/') {
        slash.set(None);
    }
    set_block_spans(blocks, id, dom::serialize_inline(el));
}

fn handle_enter(blocks: Blocks, slash: Slash, id: &str, kind: Kind, el: &Element) {
    let tail = dom::split_off_tail(el).unwrap_or_default();
    let head = dom::serialize_inline(el);
    let list = matches!(kind, Kind::Bullet | Kind::Numbered | Kind::Todo);
    if list && head.is_empty() && tail.is_empty() {
        apply_convert(blocks, slash, id.to_string(), Kind::Paragraph);
        return;
    }
    let cont = if list { kind } else { Kind::Paragraph };
    let new_id = dom::new_id();
    let nid = new_id.clone();
    let id = id.to_string();
    blocks.update(|v| {
        if let Some(i) = v.iter().position(|b| b.id() == id) {
            content::set_spans(&mut v[i], head.clone());
            v.insert(i + 1, content::make(cont, new_id.clone(), tail.clone()));
        }
    });
    dom::after_render(move || dom::focus_block(&nid, true));
}

/// Returns true if the keystroke was handled (caller calls preventDefault).
fn handle_backspace(blocks: Blocks, id: &str, el: &Element) -> bool {
    if !dom::caret_at_start(el) {
        return false;
    }
    let cur = dom::serialize_inline(el);
    let id = id.to_string();
    let mut focus_prev: Option<(String, u32)> = None;
    let mut converted = false;
    blocks.update(|v| {
        let Some(i) = v.iter().position(|b| b.id() == id) else {
            return;
        };
        if i == 0 {
            if !matches!(v[0], Block::Paragraph { .. }) {
                let bid = v[0].id().to_string();
                v[0] = content::make(Kind::Paragraph, bid, content::spans_of(&v[0]));
                converted = true;
            }
            return;
        }
        if content::is_text(&v[i - 1]) {
            let mut merged = content::spans_of(&v[i - 1]);
            let join = dom::spans_len(&merged);
            merged.extend(cur.clone());
            let prev_id = v[i - 1].id().to_string();
            content::set_spans(&mut v[i - 1], merged);
            v.remove(i);
            focus_prev = Some((prev_id, join));
        } else {
            v.remove(i - 1);
        }
    });
    if let Some((pid, join)) = focus_prev {
        dom::after_render(move || {
            if let Some(html) = blocks
                .get_untracked()
                .iter()
                .find(|b| b.id() == pid)
                .map(|b| render_inline(&content::spans_of(b)))
            {
                dom::reseed_block(&pid, &html);
            }
            dom::focus_block_offset(&pid, join);
        });
        true
    } else if converted {
        reseed_focus(blocks, id, true);
        true
    } else {
        true // at start with no prior text block: swallow so caret doesn't drift
    }
}

fn remove_block(blocks: Blocks, id: &str) {
    blocks.update(|v| {
        v.retain(|b| b.id() != id);
        if v.is_empty() {
            v.push(content::empty_paragraph());
        }
    });
}

#[component]
pub fn BlockRow(block: Block, blocks: Blocks, slash: Slash) -> impl IntoView {
    let id = block.id().to_string();

    match &block {
        Block::Code { language, code, .. } => {
            code_view(blocks, id, language.clone(), code.clone()).into_any()
        }
        Block::Divider { .. } => divider_view(blocks, id).into_any(),
        Block::Image { src, alt, .. } => {
            image_view(blocks, id, src.clone(), alt.clone()).into_any()
        }
        _ => text_view(block, blocks, slash, id).into_any(),
    }
}

fn editable_class(kind: Kind) -> &'static str {
    match kind {
        Kind::H1 => "flex-1 text-2xl font-semibold text-white outline-none",
        Kind::H2 => "flex-1 text-xl font-semibold text-white outline-none",
        Kind::H3 => "flex-1 text-lg font-semibold text-white outline-none",
        Kind::Quote => "flex-1 italic text-zinc-300 outline-none",
        _ => "flex-1 text-sm/6 text-zinc-200 outline-none",
    }
}

fn text_view(block: Block, blocks: Blocks, slash: Slash, id: String) -> impl IntoView {
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
                handle_input(blocks, slash, &id, div.unchecked_ref::<Element>());
            }
        }
    };
    let on_keydown = {
        let id = id.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            let Some(div) = node.get() else { return };
            let el = div.unchecked_ref::<Element>();
            match ev.key().as_str() {
                "Enter" if !ev.shift_key() => {
                    ev.prevent_default();
                    handle_enter(blocks, slash, &id, kind, el);
                }
                "Backspace" => {
                    if handle_backspace(blocks, &id, el) {
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
    let marker = marker_view(kind, todo_checked, callout_emoji, blocks, id.clone());

    let row_class = match kind {
        Kind::Quote => "flex gap-2 border-l-2 border-white/25 pl-3 py-0.5",
        Kind::Callout => {
            "flex gap-2 rounded-lg border border-amber-400/20 bg-amber-400/5 px-3 py-2"
        }
        Kind::Bullet | Kind::Numbered | Kind::Todo => "flex items-start gap-2 py-0.5",
        _ => "py-0.5",
    };

    let id_attr = id.clone();
    let editable = view! {
        <div
            contenteditable="true"
            node_ref=node
            data-block=id_attr
            on:input=on_input
            on:keydown=on_keydown
            class=editable_class(kind)
        ></div>
    };

    view! {
        <div class="group relative rounded-lg px-1 hover:bg-white/[0.02]">
            <div class=row_class>
                {marker}
                {editable}
            </div>
            {move || (slash.get().as_deref() == Some(id.as_str())).then(|| {
                let bid = id.clone();
                let bid2 = id.clone();
                view! {
                    <SlashMenu
                        on_pick=Callback::new(move |k| apply_convert(blocks, slash, bid.clone(), k))
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

fn marker_view(
    kind: Kind,
    checked: bool,
    emoji: String,
    blocks: Blocks,
    id: String,
) -> impl IntoView {
    match kind {
        Kind::Bullet => view! { <span class="mt-1.5 select-none text-zinc-500">"•"</span> }.into_any(),
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

fn delete_btn(blocks: Blocks, id: String) -> impl IntoView {
    view! {
        <button
            class="absolute right-1 top-1 hidden rounded p-1 text-xs text-zinc-500 \
                hover:bg-white/10 hover:text-white group-hover:block"
            title="Eliminar bloque"
            on:click=move |_| remove_block(blocks, &id)
        >
            "✕"
        </button>
    }
}

fn code_view(blocks: Blocks, id: String, language: String, code: String) -> impl IntoView {
    let (id_lang, id_code, id_del) = (id.clone(), id.clone(), id.clone());
    view! {
        <div class="group relative rounded-lg border border-white/10 bg-black/30 p-2">
            <input
                class="mb-1 w-40 bg-transparent font-mono text-xs text-zinc-400 outline-none \
                    placeholder:text-zinc-600"
                placeholder="lenguaje"
                prop:value=language
                on:input=move |e| {
                    let val = event_target_value(&e);
                    blocks.update(|v| {
                        if let Some(Block::Code { language, .. }) =
                            v.iter_mut().find(|b| b.id() == id_lang)
                        {
                            *language = val.clone();
                        }
                    });
                }
            />
            <textarea
                class="block w-full resize-y bg-transparent font-mono text-sm text-zinc-200 \
                    outline-none"
                rows="3"
                prop:value=code
                on:input=move |e| {
                    let val = event_target_value(&e);
                    blocks.update(|v| {
                        if let Some(Block::Code { code, .. }) =
                            v.iter_mut().find(|b| b.id() == id_code)
                        {
                            *code = val.clone();
                        }
                    });
                }
            ></textarea>
            {delete_btn(blocks, id_del)}
        </div>
    }
}

fn divider_view(blocks: Blocks, id: String) -> impl IntoView {
    view! {
        <div class="group relative py-2">
            <hr class="border-white/15" />
            {delete_btn(blocks, id)}
        </div>
    }
}

fn image_view(blocks: Blocks, id: String, src: String, alt: String) -> impl IntoView {
    let id_alt = id.clone();
    view! {
        <div class="group relative rounded-lg border border-white/10 p-2">
            <img src=src alt=alt.clone() class="mx-auto max-h-80 rounded" />
            <input
                class="mt-1 w-full bg-transparent text-center text-xs text-zinc-400 outline-none \
                    placeholder:text-zinc-600"
                placeholder="texto alternativo"
                prop:value=alt
                on:input=move |e| {
                    let val = event_target_value(&e);
                    blocks.update(|v| {
                        if let Some(Block::Image { alt, .. }) =
                            v.iter_mut().find(|b| b.id() == id_alt)
                        {
                            *alt = val.clone();
                        }
                    });
                }
            />
            {delete_btn(blocks, id)}
        </div>
    }
}
