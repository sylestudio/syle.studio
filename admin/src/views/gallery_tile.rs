//! One photo cell of the gallery editor grid: cover, hover overlay (delete +
//! reorder), and the inline alt-text editor with its dirty/non-empty save
//! guard. Split out of `gallery.rs` to keep that view within the LOC cap and
//! to give the tile a single, testable responsibility. Behavior is unchanged
//! from the original inline closure; parent state crosses the boundary via
//! `Callback`s (reorder/reload) and the two shared confirm signals.

use crate::api;
use crate::ui::use_toaster;
use leptos::prelude::*;
use syle_types::{Photo, UpdatePhoto};
use wasm_bindgen_futures::spawn_local;

/// Smallest JPEG variant (or any variant) for a grid thumbnail.
pub fn thumb_src(p: &Photo) -> String {
    p.variants
        .iter()
        .filter(|v| v.path.ends_with(".jpeg"))
        .min_by_key(|v| v.width)
        .or_else(|| p.variants.first())
        .map(|v| v.path.clone())
        .unwrap_or_default()
}

#[component]
pub fn PhotoTile(
    photo: Photo,
    index: usize,
    total: usize,
    confirm_photo: RwSignal<Option<String>>,
    confirm_photo_open: RwSignal<bool>,
    #[prop(into)] reorder: Callback<(usize, usize)>,
    #[prop(into)] reload: Callback<()>,
) -> impl IntoView {
    let toast = use_toaster();
    let pid = photo.id.to_string();
    let alt = RwSignal::new(photo.alt.clone());
    let alt0 = photo.alt.clone();
    let alt_dirty = Signal::derive(move || {
        let v = alt.get();
        let v = v.trim();
        !v.is_empty() && v != alt0
    });
    let pid_alt = pid.clone();
    let pid_del = pid.clone();
    let src = thumb_src(&photo);

    view! {
        <div class="reveal-item group rounded-2xl border border-white/10 \
            bg-white/4 p-1.5 transition duration-300 ease-fluid \
            hover:-translate-y-0.5 hover:border-white/20 hover:bg-white/6 \
            motion-reduce:transition-none motion-reduce:hover:translate-y-0">
            <div class="bezel-core overflow-hidden rounded-xl bg-zinc-900">
            <div class="relative aspect-square bg-zinc-800">
                {if src.is_empty() {
                    view! {
                        <div class="flex h-full w-full items-center \
                            justify-center text-xs/5 text-zinc-600">
                            "sin derivados"
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <img src=src alt=photo.alt.clone()
                            class="absolute inset-0 block h-full w-full object-cover \
                                transition duration-500 ease-fluid group-hover:scale-105 \
                                motion-reduce:transition-none motion-reduce:group-hover:scale-100" />
                    }.into_any()
                }}
                <div class="pointer-events-none absolute inset-0 \
                    bg-linear-to-t from-black/70 via-black/0 to-black/0 \
                    opacity-0 transition group-hover:opacity-100"></div>
                // top-right delete
                <div class="absolute right-2 top-2 opacity-0 transition \
                    group-hover:opacity-100">
                    {
                        let pid_set = pid_del.clone();
                        view! {
                            <button
                                class="rounded-lg bg-black/60 px-2 py-1 \
                                    text-xs font-semibold text-white \
                                    backdrop-blur hover:bg-red-600"
                                on:click=move |_| {
                                    confirm_photo.set(Some(pid_set.clone()));
                                    confirm_photo_open.set(true);
                                }>
                                "Borrar"
                            </button>
                        }
                    }
                </div>
                // bottom reorder controls
                <div class="absolute inset-x-2 bottom-2 flex gap-1 \
                    opacity-0 transition group-hover:opacity-100">
                    <button
                        class="rounded-lg bg-black/60 px-2 py-1 text-xs \
                            text-white backdrop-blur hover:bg-white/20 \
                            disabled:opacity-30"
                        disabled={index == 0}
                        on:click=move |_| reorder.run((index, index.wrapping_sub(1)))>"←"</button>
                    <button
                        class="rounded-lg bg-black/60 px-2 py-1 text-xs \
                            text-white backdrop-blur hover:bg-white/20 \
                            disabled:opacity-30"
                        disabled={index + 1 >= total}
                        on:click=move |_| reorder.run((index, index + 1))>"→"</button>
                </div>
            </div>
            <div class="flex gap-2 p-2">
                <input
                    class="block w-full rounded-lg border-0 \
                        bg-white/5 px-2.5 py-1 text-xs/5 text-white \
                        placeholder:text-zinc-600 focus:outline-2 \
                        focus:-outline-offset-2 focus:outline-blue-500"
                    prop:value=alt
                    placeholder="Texto alternativo"
                    on:input=move |e| alt.set(event_target_value(&e)) />
                <button
                    class="shrink-0 rounded-lg px-2 py-1 text-xs/5 \
                        font-medium text-zinc-400 hover:bg-white/10 \
                        hover:text-white transition-colors \
                        disabled:opacity-30 disabled:pointer-events-none"
                    disabled=Signal::derive(move || !alt_dirty.get())
                    on:click=move |_| {
                        let id = pid_alt.clone();
                        let body = UpdatePhoto {
                            alt: Some(alt.get().trim().to_string()),
                            position: None,
                        };
                        spawn_local(async move {
                            match api::update_photo(&id, &body).await {
                                Ok(_) => { toast.ok("Alt guardado"); reload.run(()); }
                                Err(_) => toast.err("Error al guardar alt"),
                            }
                        });
                    }>"Guardar"</button>
            </div>
            </div>
        </div>
    }
}
