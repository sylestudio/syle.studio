//! Canvas engine for the image editor: load a `File` into an `HtmlImageElement`,
//! paint the rotated/flipped "stage" into the on-screen display canvas (CSS
//! `filter` handles brightness/contrast/preset live, so this only re-runs on a
//! rotate/flip), and bake the final cropped + filtered PNG `Blob`.
//!
//! Crop coordinates live in *stage space* (the rotated image), the same space
//! the display canvas shows — so the overlay maps 1:1 and `bake` slices the
//! stage with the crop rect verbatim.

use std::f64::consts::FRAC_PI_2;
use syle_imageedit::{bake_dims, rotated_dims, CropRect, BAKE_MAX_LONG};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

/// Everything `bake` needs, resolved from the editor's reactive state.
pub struct Bake {
    pub crop: CropRect,
    pub quarters: u8,
    pub flip_h: bool,
    pub flip_v: bool,
    /// CSS filter string (adjustments + B/N or Fade preset); `""` → none.
    pub filter: String,
    /// Vignette is a compositing pass, not a CSS filter.
    pub vignette: bool,
}

fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

fn new_canvas(w: u32, h: u32) -> Result<HtmlCanvasElement, JsValue> {
    let c: HtmlCanvasElement = document().create_element("canvas")?.dyn_into()?;
    c.set_width(w.max(1));
    c.set_height(h.max(1));
    Ok(c)
}

fn ctx2d(c: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, JsValue> {
    let obj = c
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("no 2d context"))?;
    Ok(obj.dyn_into()?)
}

/// Decode `file` into an `HtmlImageElement`, resolving once it has loaded.
/// Returns the element and the object URL (revoke it when done).
pub async fn load_image(file: &web_sys::File) -> Result<(HtmlImageElement, String), JsValue> {
    let url = web_sys::Url::create_object_url_with_blob(file)?;
    let img = HtmlImageElement::new()?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let onload = Closure::once_into_js(move || {
            let _ = resolve.call0(&JsValue::NULL);
        });
        let onerror = Closure::once_into_js(move || {
            let _ = reject.call0(&JsValue::NULL);
        });
        img.set_onload(Some(onload.unchecked_ref()));
        img.set_onerror(Some(onerror.unchecked_ref()));
    });
    img.set_src(&url);
    wasm_bindgen_futures::JsFuture::from(promise).await?;
    Ok((img, url))
}

/// Stage dimensions (the rotated full image) for an image at `quarters`.
pub fn stage_dims(img: &HtmlImageElement, quarters: u8) -> (f64, f64) {
    let (w, h) = rotated_dims(img.natural_width(), img.natural_height(), quarters);
    (w as f64, h as f64)
}

/// Draw the rotated/flipped full image into `canvas`, fitting the stage inside
/// `max_w × max_h` without upscaling. Returns `(display_w, display_h, scale)`
/// where `scale` is display-px per stage-px (the crop overlay multiplies by it).
pub fn render_display(
    canvas: &HtmlCanvasElement,
    img: &HtmlImageElement,
    quarters: u8,
    flip_h: bool,
    flip_v: bool,
    max_w: f64,
    max_h: f64,
) -> Result<(f64, f64, f64), JsValue> {
    let (sw, sh) = stage_dims(img, quarters);
    let scale = (max_w / sw).min(max_h / sh).clamp(0.0001, 1.0);
    let (dw, dh) = (sw * scale, sh * scale);
    canvas.set_width(dw.round() as u32);
    canvas.set_height(dh.round() as u32);
    let ctx = ctx2d(canvas)?;
    paint(&ctx, img, quarters, flip_h, flip_v, dw, dh)?;
    Ok((dw, dh, scale))
}

/// Paint the rotated/flipped image to fill a `w × h` context (no filter — the
/// display canvas filters via CSS; the bake stage sets `ctx.filter` first).
fn paint(
    ctx: &CanvasRenderingContext2d,
    img: &HtmlImageElement,
    quarters: u8,
    flip_h: bool,
    flip_v: bool,
    w: f64,
    h: f64,
) -> Result<(), JsValue> {
    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)?;
    ctx.clear_rect(0.0, 0.0, w, h);
    ctx.translate(w / 2.0, h / 2.0)?;
    ctx.rotate(quarters as f64 * FRAC_PI_2)?;
    ctx.scale(if flip_h { -1.0 } else { 1.0 }, if flip_v { -1.0 } else { 1.0 })?;
    // Pre-rotation drawn size: swap on a quarter turn so it fills the stage.
    let (idw, idh) = if quarters % 2 == 1 { (h, w) } else { (w, h) };
    ctx.draw_image_with_html_image_element_and_dw_and_dh(img, -idw / 2.0, -idh / 2.0, idw, idh)?;
    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)?;
    Ok(())
}

/// Render the final image and resolve a PNG `Blob`. Stage is rendered at native
/// resolution with `ctx.filter` baked in, then the crop rect is sliced and
/// capped to [`BAKE_MAX_LONG`]; PNG keeps it lossless (the server re-encodes).
pub async fn bake(img: &HtmlImageElement, b: &Bake) -> Result<Blob, JsValue> {
    let (sw, sh) = stage_dims(img, b.quarters);
    let stage = new_canvas(sw as u32, sh as u32)?;
    let sctx = ctx2d(&stage)?;
    sctx.set_filter(if b.filter.is_empty() { "none" } else { &b.filter });
    paint(&sctx, img, b.quarters, b.flip_h, b.flip_v, sw, sh)?;
    sctx.set_filter("none");

    let (ow, oh) = bake_dims(b.crop.w, b.crop.h, BAKE_MAX_LONG);
    let out = new_canvas(ow, oh)?;
    let octx = ctx2d(&out)?;
    octx.draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
        &stage, b.crop.x, b.crop.y, b.crop.w, b.crop.h, 0.0, 0.0, ow as f64, oh as f64,
    )?;
    if b.vignette {
        apply_vignette(&octx, ow as f64, oh as f64)?;
    }
    canvas_to_png(&out).await
}

/// Darken the corners with a radial gradient (the brand's cinematic falloff).
fn apply_vignette(ctx: &CanvasRenderingContext2d, w: f64, h: f64) -> Result<(), JsValue> {
    let (cx, cy) = (w / 2.0, h / 2.0);
    let inner = cx.min(cy) * 0.55;
    let outer = (cx * cx + cy * cy).sqrt();
    let grad = ctx.create_radial_gradient(cx, cy, inner, cx, cy, outer)?;
    grad.add_color_stop(0.0, "rgba(0,0,0,0)")?;
    grad.add_color_stop(1.0, "rgba(0,0,0,0.55)")?;
    ctx.set_fill_style_canvas_gradient(&grad);
    ctx.fill_rect(0.0, 0.0, w, h);
    Ok(())
}

async fn canvas_to_png(canvas: &HtmlCanvasElement) -> Result<Blob, JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let cb = Closure::once_into_js(move |blob: JsValue| {
            let _ = resolve.call1(&JsValue::NULL, &blob);
        });
        let _ = canvas.to_blob_with_type(cb.unchecked_ref(), "image/png");
    });
    let v = wasm_bindgen_futures::JsFuture::from(promise).await?;
    v.dyn_into::<Blob>()
}
