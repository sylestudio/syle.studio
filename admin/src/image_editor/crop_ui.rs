//! Draggable crop overlay. Renders over the display canvas in stage-display
//! space; grip drags go through the host-tested [`CropRect::drag`] so the fiddly
//! geometry is the part that has unit tests. Deltas are taken in client pixels
//! (which cancel out the container offset) and divided by the display scale to
//! reach stage pixels.

use leptos::prelude::*;
use syle_imageedit::{CropRect, Handle};
use web_sys::PointerEvent;

/// Active drag: the grip, the pointer's start client position, and the crop as
/// it was when the drag began (each move recomputes from this, not cumulatively).
type Drag = (Handle, f64, f64, CropRect);

#[component]
pub fn CropOverlay(
    crop: RwSignal<CropRect>,
    /// Stage (rotated source) dimensions in source px.
    stage: Signal<(f64, f64)>,
    /// Display px per stage px.
    scale: Signal<f64>,
    /// Locked aspect ratio (w/h), or `None` for free crop.
    ratio: Signal<Option<f64>>,
) -> impl IntoView {
    let active = RwSignal::new(None::<Drag>);
    let container = NodeRef::<leptos::html::Div>::new();

    let on_move = move |ev: PointerEvent| {
        let Some((h, sx, sy, start)) = active.get_untracked() else {
            return;
        };
        let sc = scale.get_untracked().max(0.0001);
        let (sw, sh) = stage.get_untracked();
        let dx = (ev.client_x() as f64 - sx) / sc;
        let dy = (ev.client_y() as f64 - sy) / sc;
        crop.set(start.drag(h, dx, dy, ratio.get_untracked(), sw, sh));
    };
    let end = move |_ev: PointerEvent| active.set(None);

    // Each grip records the start state and captures the pointer to the
    // container so the drag survives the cursor leaving the grip.
    let grip = move |h: Handle| {
        move |ev: PointerEvent| {
            ev.prevent_default();
            ev.stop_propagation();
            active.set(Some((h, ev.client_x() as f64, ev.client_y() as f64, crop.get_untracked())));
            if let Some(el) = container.get() {
                let _ = el.set_pointer_capture(ev.pointer_id());
            }
        }
    };

    // size+look inline (no dynamic Tailwind classes for the scanner to miss).
    const DOT: &str = "position:absolute;width:12px;height:12px;background:#fff;\
        border:1px solid #18181b;border-radius:2px;";

    view! {
        <div
            node_ref=container
            class="absolute inset-0 overflow-hidden"
            style="touch-action:none"
            on:pointermove=on_move
            on:pointerup=end
            on:pointercancel=end
        >
            {move || {
                let c = crop.get();
                let sc = scale.get();
                let (l, t, w, h) = (c.x * sc, c.y * sc, c.w * sc, c.h * sc);
                let rect = format!(
                    "position:absolute;left:{l}px;top:{t}px;width:{w}px;height:{h}px;\
                     box-shadow:0 0 0 9999px rgba(0,0,0,0.55);outline:1px solid rgba(255,255,255,0.9);\
                     cursor:move"
                );
                view! {
                    <div style=rect on:pointerdown=grip(Handle::Move)>
                        <div style=format!("{DOT}left:-6px;top:-6px;cursor:nwse-resize") on:pointerdown=grip(Handle::Nw)></div>
                        <div style=format!("{DOT}left:calc(50% - 6px);top:-6px;cursor:ns-resize") on:pointerdown=grip(Handle::N)></div>
                        <div style=format!("{DOT}right:-6px;top:-6px;cursor:nesw-resize") on:pointerdown=grip(Handle::Ne)></div>
                        <div style=format!("{DOT}right:-6px;top:calc(50% - 6px);cursor:ew-resize") on:pointerdown=grip(Handle::E)></div>
                        <div style=format!("{DOT}right:-6px;bottom:-6px;cursor:nwse-resize") on:pointerdown=grip(Handle::Se)></div>
                        <div style=format!("{DOT}left:calc(50% - 6px);bottom:-6px;cursor:ns-resize") on:pointerdown=grip(Handle::S)></div>
                        <div style=format!("{DOT}left:-6px;bottom:-6px;cursor:nesw-resize") on:pointerdown=grip(Handle::Sw)></div>
                        <div style=format!("{DOT}left:-6px;top:calc(50% - 6px);cursor:ew-resize") on:pointerdown=grip(Handle::W)></div>
                    </div>
                }
            }}
        </div>
    }
}
