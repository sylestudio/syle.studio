//! Thin `web-sys` helpers for the block editor: pure DOM ↔ data transforms and
//! caret plumbing. They own no UI (the editing surface is Leptos), so they stay
//! a leaf utility. Offsets are counted in UTF-16 units to match the DOM/Range
//! API; for the Latin/Spanish text this editor handles that equals characters.

use std::cell::RefCell;
use syle_types::{Mark, Span};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, Node, Window};

const TEXT_NODE: u16 = 3;
const ELEMENT_NODE: u16 = 1;

fn win() -> Option<Window> {
    web_sys::window()
}
fn doc() -> Option<Document> {
    win()?.document()
}

/// A stable block id from the platform RNG (`crypto.randomUUID`).
pub fn new_id() -> String {
    win()
        .and_then(|w| w.crypto().ok())
        .map(|c| c.random_uuid())
        .unwrap_or_else(|| "b-unknown".into())
}

/// Run `f` after the current render flushes (next macrotask).
pub fn after_render(f: impl FnOnce() + 'static) {
    gloo_timers::callback::Timeout::new(0, f).forget();
}

fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}

fn push_span(out: &mut Vec<Span>, text: String, marks: &[Mark]) {
    if let Some(last) = out.last_mut() {
        if last.marks == marks {
            last.text.push_str(&text);
            return;
        }
    }
    out.push(Span {
        text,
        marks: marks.to_vec(),
    });
}

fn add_mark(marks: &mut Vec<Mark>, m: Mark) {
    if let Mark::Link { .. } = m {
        marks.retain(|x| !matches!(x, Mark::Link { .. }));
        marks.push(m);
    } else if !marks.contains(&m) {
        marks.push(m);
    }
}

fn walk(node: &Node, marks: &[Mark], out: &mut Vec<Span>) {
    let children = node.child_nodes();
    for i in 0..children.length() {
        let Some(child) = children.item(i) else { continue };
        match child.node_type() {
            TEXT_NODE => {
                let text = child.text_content().unwrap_or_default();
                if !text.is_empty() {
                    push_span(out, text, marks);
                }
            }
            ELEMENT_NODE => {
                let Some(elem) = child.dyn_ref::<Element>() else { continue };
                let tag = elem.tag_name().to_lowercase();
                let mut m = marks.to_vec();
                match tag.as_str() {
                    "b" | "strong" => add_mark(&mut m, Mark::Bold),
                    "i" | "em" => add_mark(&mut m, Mark::Italic),
                    "code" => add_mark(&mut m, Mark::Code),
                    "s" | "strike" | "del" => add_mark(&mut m, Mark::Strike),
                    "a" => {
                        if let Some(href) = elem.get_attribute("href") {
                            add_mark(&mut m, Mark::Link { href });
                        }
                    }
                    "br" => {}
                    _ => {}
                }
                walk(elem.unchecked_ref::<Node>(), &m, out);
            }
            _ => {}
        }
    }
}

/// Serialize a contenteditable element's children into inline spans.
pub fn serialize_inline(el: &Element) -> Vec<Span> {
    let mut out = Vec::new();
    walk(el.unchecked_ref::<Node>(), &[], &mut out);
    out
}

/// UTF-16 offset of the (collapsed) caret within `el`, or 0.
pub fn caret_offset(el: &Element) -> u32 {
    (|| {
        let sel = win()?.get_selection().ok()??;
        if sel.range_count() == 0 {
            return None;
        }
        let range = sel.get_range_at(0).ok()?;
        let document = doc()?;
        let pre = document.create_range().ok()?;
        pre.select_node_contents(el.unchecked_ref::<Node>()).ok()?;
        pre.set_end(&range.end_container().ok()?, range.end_offset().ok()?)
            .ok()?;
        let frag = pre.clone_contents().ok()?;
        let tmp = document.create_element("div").ok()?;
        tmp.append_child(frag.unchecked_ref::<Node>()).ok()?;
        Some(utf16_len(&tmp.text_content().unwrap_or_default()))
    })()
    .unwrap_or(0)
}

/// Split `el` at the caret: everything after the caret is removed from `el`'s
/// DOM (so `el` now holds only the head) and returned as spans (the tail).
pub fn split_off_tail(el: &Element) -> Option<Vec<Span>> {
    let sel = win()?.get_selection().ok()??;
    if sel.range_count() == 0 {
        return None;
    }
    let range = sel.get_range_at(0).ok()?;
    let tail = doc()?.create_range().ok()?;
    tail.select_node_contents(el.unchecked_ref::<Node>()).ok()?;
    tail.set_start(&range.end_container().ok()?, range.end_offset().ok()?)
        .ok()?;
    let frag = tail.extract_contents().ok()?;
    let tmp = doc()?.create_element("div").ok()?;
    tmp.append_child(frag.unchecked_ref::<Node>()).ok()?;
    Some(serialize_inline(&tmp))
}

fn find_text_at(node: &Node, target: u32, acc: &mut u32) -> Option<(Node, u32)> {
    let children = node.child_nodes();
    for i in 0..children.length() {
        let Some(child) = children.item(i) else { continue };
        match child.node_type() {
            TEXT_NODE => {
                let len = utf16_len(&child.text_content().unwrap_or_default());
                if *acc + len >= target {
                    return Some((child, target - *acc));
                }
                *acc += len;
            }
            ELEMENT_NODE => {
                if let Some(hit) = find_text_at(&child, target, acc) {
                    return Some(hit);
                }
            }
            _ => {}
        }
    }
    None
}

/// Place a collapsed caret at UTF-16 offset `target` within `el`.
pub fn set_caret(el: &Element, target: u32) {
    let _ = (|| {
        let window = win()?;
        let document = doc()?;
        let sel = window.get_selection().ok()??;
        let range = document.create_range().ok()?;
        let mut acc = 0;
        if let Some((node, off)) = find_text_at(el.unchecked_ref::<Node>(), target, &mut acc) {
            range.set_start(&node, off).ok()?;
            range.collapse_with_to_start(true);
        } else {
            range.select_node_contents(el.unchecked_ref::<Node>()).ok()?;
            range.collapse_with_to_start(false);
        }
        sel.remove_all_ranges().ok()?;
        sel.add_range(&range).ok()?;
        Some(())
    })();
}

/// True when the caret is collapsed at the very start of `el`.
pub fn caret_at_start(el: &Element) -> bool {
    let collapsed = win()
        .and_then(|w| w.get_selection().ok().flatten())
        .map(|s| s.is_collapsed())
        .unwrap_or(false);
    collapsed && caret_offset(el) == 0
}

/// UTF-16 length of inline spans (caret join offset after a merge).
pub fn spans_len(spans: &[Span]) -> u32 {
    spans.iter().map(|s| utf16_len(&s.text)).sum()
}

fn query_block(id: &str) -> Option<Element> {
    doc()?
        .query_selector(&format!("[data-block=\"{id}\"]"))
        .ok()?
}

/// Focus the contenteditable for block `id`, placing the caret at start/end.
pub fn focus_block(id: &str, at_start: bool) {
    if let Some(el) = query_block(id) {
        if let Some(h) = el.dyn_ref::<HtmlElement>() {
            h.focus().ok();
        }
        set_caret(&el, if at_start { 0 } else { u32::MAX });
    }
}

/// Focus block `id` and place the caret at UTF-16 offset `at`.
pub fn focus_block_offset(id: &str, at: u32) {
    if let Some(el) = query_block(id) {
        if let Some(h) = el.dyn_ref::<HtmlElement>() {
            h.focus().ok();
        }
        set_caret(&el, at);
    }
}

/// Re-seed block `id`'s DOM from rendered HTML (used after a structural edit
/// that didn't re-mount the row, so the DOM still shows stale content).
pub fn reseed_block(id: &str, html: &str) {
    if let Some(el) = query_block(id) {
        el.set_inner_html(html);
    }
}

/// Replace an element's inner HTML (used to re-seed a block after a structural
/// edit; never called on the focused-typing path).
pub fn set_html(el: &Element, html: &str) {
    el.set_inner_html(html);
}

/// The focused contenteditable block element, if any.
pub fn active_editable() -> Option<Element> {
    let el = doc()?.active_element()?;
    el.has_attribute("data-block").then_some(el)
}

/// `document.execCommand(...)` — called via reflection because the web-sys
/// binding for this legacy API isn't exposed in this version. Args are passed
/// positionally; missing trailing args default in JS.
fn exec_command(args: &[wasm_bindgen::JsValue]) {
    let _ = (|| {
        let document = doc()?;
        let func = js_sys::Reflect::get(&document, &"execCommand".into()).ok()?;
        let func = func.dyn_into::<js_sys::Function>().ok()?;
        let argv = js_sys::Array::new();
        for a in args {
            argv.push(a);
        }
        js_sys::Reflect::apply(&func, &document, &argv).ok()?;
        Some(())
    })();
}

/// Force tag-based formatting (`<b>`/`<i>`) instead of inline styles, so the
/// serializer sees marks as elements. Call once at editor init.
pub fn use_tag_formatting() {
    use wasm_bindgen::JsValue;
    exec_command(&[
        JsValue::from_str("styleWithCSS"),
        JsValue::FALSE,
        JsValue::from_str("false"),
    ]);
}

/// Run a `document.execCommand` formatting toggle on the current selection.
pub fn exec(command: &str) {
    use wasm_bindgen::JsValue;
    exec_command(&[
        JsValue::from_str(command),
        JsValue::FALSE,
        JsValue::from_str(""),
    ]);
}

thread_local! {
    /// Selection captured when the link button is pressed, so the `<a>` can wrap
    /// it after focus has moved away to the URL input.
    static SAVED_RANGE: RefCell<Option<web_sys::Range>> = const { RefCell::new(None) };
}

/// Capture the live selection for a later `create_link`. Returns true when the
/// selection is a non-empty range (i.e. there is text to link).
pub fn save_selection() -> bool {
    let range = (|| {
        let sel = win()?.get_selection().ok()??;
        (sel.range_count() > 0).then(|| sel.get_range_at(0).ok())?
    })();
    let has_text = range.as_ref().map(|r| !r.collapsed()).unwrap_or(false);
    SAVED_RANGE.with(|c| *c.borrow_mut() = range);
    has_text
}

/// Wrap the saved selection in `<a href=…>`. No-op (returns false) if there is
/// no saved range, it is collapsed, or it crosses element boundaries
/// (`surroundContents` only wraps a single inline context — same limit as
/// `wrap_inline_code`). The href is assumed already normalized by the caller.
pub fn create_link(href: &str) -> bool {
    SAVED_RANGE.with(|c| {
        let Some(range) = c.borrow().clone() else {
            return false;
        };
        if range.collapsed() {
            return false;
        }
        (|| {
            let a = doc()?.create_element("a").ok()?;
            a.set_attribute("href", href).ok()?;
            range.surround_contents(a.unchecked_ref::<Node>()).ok()?;
            Some(())
        })()
        .is_some()
    })
}

/// Serialize block `id`'s current DOM into spans. Used after a formatting
/// command when the contenteditable may no longer be the active element.
pub fn serialize_block(id: &str) -> Option<Vec<Span>> {
    query_block(id).map(|el| serialize_inline(&el))
}

/// Wrap the current (non-collapsed) selection in an inline `<code>` element.
pub fn wrap_inline_code() {
    let _ = (|| {
        let sel = win()?.get_selection().ok()??;
        if sel.range_count() == 0 {
            return None;
        }
        let range = sel.get_range_at(0).ok()?;
        if range.collapsed() {
            return None;
        }
        let code = doc()?.create_element("code").ok()?;
        range.surround_contents(code.unchecked_ref::<Node>()).ok()?;
        Some(())
    })();
}
