//! In-browser image editor (Fase A): edit-on-upload, baked client-side and sent
//! through the existing ingest endpoints. Shared by galleries and the blog editor.
//!
//! The modal takes a picked `File` and hands back a baked PNG `Blob` via
//! `on_baked`; the caller wires that into its existing multipart upload. Preview
//! is instant — CSS `filter` on the display canvas for adjustments/B&W/Fade, a
//! gradient overlay for the vignette, and the crop overlay for the crop rect —
//! while [`canvas::bake`] renders the authoritative pixels only on commit.

mod canvas;
mod crop_ui;

use crate::ui::{use_toaster, Button};
use crop_ui::CropOverlay;
use leptos::prelude::*;
use syle_imageedit::{css_filter, Adjust, CropRect, Preset};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Blob, File};

/// Preview area the stage is fit into (px). Bake always runs at native res.
const PREVIEW_W: f64 = 660.0;
const PREVIEW_H: f64 = 440.0;

const CHIP: &str = "rounded-md px-2.5 py-1 text-xs font-medium transition-colors";
const CHIP_ON: &str = "rounded-md px-2.5 py-1 text-xs font-medium bg-white text-zinc-950";
const CHIP_OFF: &str =
    "rounded-md px-2.5 py-1 text-xs font-medium bg-white/5 text-zinc-300 hover:bg-white/10";

fn chip(on: bool) -> &'static str {
    if on {
        CHIP_ON
    } else {
        CHIP_OFF
    }
}

#[component]
pub fn ImageEditorModal(
    file: File,
    #[prop(into)] on_baked: Callback<Blob>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let toast = use_toaster();

    let brightness = RwSignal::new(100u16);
    let contrast = RwSignal::new(100u16);
    let saturation = RwSignal::new(100u16);
    let quarters = RwSignal::new(0u8);
    let flip_h = RwSignal::new(false);
    let flip_v = RwSignal::new(false);
    let preset = RwSignal::new(None::<Preset>);
    let ratio = RwSignal::new(None::<f64>);
    let crop = RwSignal::new(CropRect::full(1.0, 1.0));
    let stage = RwSignal::new((1.0, 1.0));
    let scale = RwSignal::new(1.0);
    let loaded = RwSignal::new(false);
    let busy = RwSignal::new(false);

    // JS handles (image + object URL) are not `Send`; park them in Leptos
    // local storage so effects can hold the `Copy` handle.
    let store = StoredValue::new_local(None::<(web_sys::HtmlImageElement, String)>);
    let file = StoredValue::new_local(file);
    let display = NodeRef::<leptos::html::Canvas>::new();

    // Load the picked file once, then flip `loaded` to mount the preview.
    Effect::new(move |_| {
        if loaded.get_untracked() {
            return;
        }
        let f = file.get_value();
        spawn_local(async move {
            match canvas::load_image(&f).await {
                Ok((img, url)) => {
                    let (sw, sh) = canvas::stage_dims(&img, 0);
                    stage.set((sw, sh));
                    crop.set(CropRect::full(sw, sh));
                    scale.set((PREVIEW_W / sw).min(PREVIEW_H / sh).clamp(0.0001, 1.0));
                    store.set_value(Some((img, url)));
                    loaded.set(true);
                }
                Err(_) => {
                    toast.err("No se pudo abrir la imagen");
                    on_cancel.run(());
                }
            }
        });
    });

    // Re-paint the display canvas on load + any rotate/flip. Crop/adjust changes
    // never re-paint (crop is an overlay; adjustments are CSS).
    Effect::new(move |_| {
        let (q, fh, fv) = (quarters.get(), flip_h.get(), flip_v.get());
        let canvas_el = display.get();
        if !loaded.get_untracked() {
            return;
        }
        store.with_value(|s| {
            if let (Some(c), Some((img, _))) = (canvas_el.as_ref(), s.as_ref()) {
                if let Ok((_, _, sc)) = canvas::render_display(c, img, q, fh, fv, PREVIEW_W, PREVIEW_H) {
                    scale.set(sc);
                }
            }
        });
    });

    on_cleanup(move || {
        store.try_with_value(|s| {
            if let Some((_, url)) = s {
                let _ = web_sys::Url::revoke_object_url(url);
            }
        });
    });

    let rotate = move |_| {
        let (sw, sh) = stage.get_untracked();
        stage.set((sh, sw));
        crop.set(CropRect::full(sh, sw));
        ratio.set(None);
        quarters.update(|q| *q = (*q + 1) % 4);
    };

    let pick_ratio = move |val: Option<f64>| {
        ratio.set(val);
        if let Some(r) = val {
            let (sw, sh) = stage.get_untracked();
            crop.update(|c| *c = c.with_ratio(Some(r), sw, sh));
        }
    };

    let css = Signal::derive(move || {
        let adj = Adjust {
            brightness: brightness.get(),
            contrast: contrast.get(),
            saturation: saturation.get(),
        };
        css_filter(&adj, preset.get())
    });

    let disp = Signal::derive(move || {
        let (sw, sh) = stage.get();
        let sc = scale.get();
        (sw * sc, sh * sc)
    });

    let apply = move |_| {
        if busy.get_untracked() {
            return;
        }
        let img = store.with_value(|s| s.as_ref().map(|(i, _)| i.clone()));
        let Some(img) = img else { return };
        {
            let p = preset.get_untracked();
            let adj = Adjust {
                brightness: brightness.get_untracked(),
                contrast: contrast.get_untracked(),
                saturation: saturation.get_untracked(),
            };
            let b = canvas::Bake {
                crop: crop.get_untracked(),
                quarters: quarters.get_untracked(),
                flip_h: flip_h.get_untracked(),
                flip_v: flip_v.get_untracked(),
                filter: css_filter(&adj, p),
                vignette: matches!(p, Some(Preset::Vignette)),
            };
            busy.set(true);
            spawn_local(async move {
                match canvas::bake(&img, &b).await {
                    Ok(blob) => on_baked.run(blob),
                    Err(_) => {
                        toast.err("No se pudo procesar la imagen");
                        busy.set(false);
                    }
                }
            });
        }
    };

    let slider = move |label: &'static str, sig: RwSignal<u16>| {
        view! {
            <label class="flex items-center gap-3 text-xs text-zinc-400">
                <span class="w-16 shrink-0">{label}</span>
                <input
                    type="range" min="50" max="150" class="flex-1 accent-white"
                    prop:value=move || sig.get().to_string()
                    on:input=move |e| sig.set(event_target_value(&e).parse().unwrap_or(100))
                />
                <span class="w-9 shrink-0 text-right tabular-nums">{move || sig.get()}</span>
            </label>
        }
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="absolute inset-0 bg-black/70 backdrop-blur-sm"></div>
            <div class="relative flex max-h-[92vh] w-full max-w-3xl flex-col gap-4 overflow-y-auto \
                rounded-2xl border border-white/10 bg-zinc-900 p-5 shadow-2xl">
                <h2 class="text-base/7 font-semibold text-white">"Editar imagen"</h2>

                <div class="flex items-center justify-center rounded-lg border border-white/10 \
                    bg-black/40" style="min-height:440px">
                    {move || if loaded.get() {
                        let (dw, dh) = disp.get();
                        view! {
                            <div class="relative" style=format!("width:{dw}px;height:{dh}px")>
                                <canvas node_ref=display class="block h-full w-full"
                                    style:filter=move || {
                                        let f = css.get();
                                        if f.is_empty() { "none".to_string() } else { f }
                                    }
                                ></canvas>
                                {move || matches!(preset.get(), Some(Preset::Vignette)).then(|| view! {
                                    <div class="pointer-events-none absolute inset-0"
                                        style="background:radial-gradient(ellipse at center, \
                                            rgba(0,0,0,0) 45%, rgba(0,0,0,0.55) 100%)"></div>
                                })}
                                <CropOverlay crop=crop stage=stage.into() scale=scale.into() ratio=ratio.into() />
                            </div>
                        }.into_any()
                    } else {
                        view! { <p class="text-sm/6 text-zinc-400">"Cargando…"</p> }.into_any()
                    }}
                </div>

                // Transform + crop-ratio row
                <div class="flex flex-wrap items-center gap-2">
                    <button class=CHIP.to_owned() + " bg-white/5 text-zinc-200 hover:bg-white/10"
                        on:click=rotate>"Rotar 90°"</button>
                    <button class=move || chip(flip_h.get())
                        on:click=move |_| flip_h.update(|v| *v = !*v)>"Voltear ⇄"</button>
                    <button class=move || chip(flip_v.get())
                        on:click=move |_| flip_v.update(|v| *v = !*v)>"Voltear ⇅"</button>
                    <span class="mx-1 h-4 w-px bg-white/10"></span>
                    {[("Libre", None), ("1:1", Some(1.0)), ("4:5", Some(0.8)), ("16:9", Some(16.0 / 9.0))]
                        .into_iter()
                        .map(|(lbl, val)| view! {
                            <button class=move || chip(ratio.get() == val)
                                on:click=move |_| pick_ratio(val)>{lbl}</button>
                        }).collect_view()}
                </div>

                // Adjustment sliders
                <div class="space-y-2">
                    {slider("Brillo", brightness)}
                    {slider("Contraste", contrast)}
                    {slider("Saturación", saturation)}
                </div>

                // Brand presets
                <div class="flex flex-wrap items-center gap-2">
                    <span class="w-16 shrink-0 text-xs text-zinc-400">"Filtro"</span>
                    {[("Ninguno", None), ("B&N", Some(Preset::Bw)), ("Fade", Some(Preset::Fade)),
                      ("Viñeta", Some(Preset::Vignette))]
                        .into_iter()
                        .map(|(lbl, val)| view! {
                            <button class=move || chip(preset.get() == val)
                                on:click=move |_| preset.set(val)>{lbl}</button>
                        }).collect_view()}
                </div>

                <div class="mt-1 flex justify-end gap-2 border-t border-white/10 pt-4">
                    <Button kind="plain" on:click=move |_| on_cancel.run(())>"Cancelar"</Button>
                    <Button disabled=Signal::derive(move || busy.get() || !loaded.get())
                        on:click=apply>
                        {move || if busy.get() { "Procesando…" } else { "Aplicar y subir" }}
                    </Button>
                </div>
            </div>
        </div>
    }
}
