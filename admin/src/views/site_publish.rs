//! "Publicar al sitio" — trigger a rebuild of the static public site and show
//! honest, state-driven progress. Polling runs in a plain async loop (never a
//! reactive cycle): it writes `status`/`active`/`elapsed`, the view only reads
//! them.

use crate::api::{self, ApiError};
use crate::ui::{use_toaster, Button};
use leptos::prelude::*;
use syle_types::{SiteBuildState, SiteBuildStatus};
use wasm_bindgen_futures::spawn_local;

/// Progress width per state — stepped, tied to the real CI run (no fake %).
fn pct(state: SiteBuildState) -> u32 {
    match state {
        SiteBuildState::Queued => 25,
        SiteBuildState::Building => 70,
        SiteBuildState::Done => 100,
        SiteBuildState::Failed => 100,
        _ => 0,
    }
}

fn label(state: SiteBuildState) -> &'static str {
    match state {
        SiteBuildState::Queued => "Encolado…",
        SiteBuildState::Building => "Construyendo…",
        SiteBuildState::Done => "Publicado ✓",
        SiteBuildState::Failed => "Error al publicar",
        SiteBuildState::Idle => "Listo para publicar",
        SiteBuildState::Unconfigured => "No configurado",
    }
}

#[component]
pub fn SitePublish() -> impl IntoView {
    let toast = use_toaster();
    let status = RwSignal::new(None::<SiteBuildStatus>);
    let active = RwSignal::new(false);
    let elapsed = RwSignal::new(0u32);

    // Poll every ~2s until the run we started goes terminal. `seen_active` guards
    // the stale-run race: only trust Done/Failed after first seeing Queued/Building.
    let start_polling = move || {
        if active.get_untracked() {
            return;
        }
        active.set(true);
        elapsed.set(0);
        spawn_local(async move {
            let mut seen_active = false;
            loop {
                gloo_timers::future::TimeoutFuture::new(1000).await;
                elapsed.update(|e| *e += 1);
                if elapsed.get_untracked().is_multiple_of(2) {
                    if let Ok(s) = api::site_status().await {
                        let st = s.state;
                        status.set(Some(s));
                        if matches!(st, SiteBuildState::Queued | SiteBuildState::Building) {
                            seen_active = true;
                        } else if seen_active
                            && matches!(st, SiteBuildState::Done | SiteBuildState::Failed)
                        {
                            // Only Done/Failed are terminal. A transient GitHub
                            // error makes `site_status` fall back to `Idle`; treat
                            // that as "keep polling", not "finished" (the 5-min
                            // timeout below is the real backstop).
                            active.set(false);
                            if st == SiteBuildState::Done {
                                toast.ok("Sitio publicado ✓");
                            } else {
                                toast.err("Falló la publicación");
                            }
                            break;
                        }
                    }
                }
                if elapsed.get_untracked() > 300 {
                    active.set(false); // safety: never spin forever
                    break;
                }
            }
        });
    };

    // Initial fetch — reflects current state, and joins an in-flight build.
    Effect::new(move |_| {
        if status.get_untracked().is_some() {
            return;
        }
        spawn_local(async move {
            if let Ok(s) = api::site_status().await {
                let joining = s.is_active();
                status.set(Some(s));
                if joining {
                    start_polling();
                }
            }
        });
    });

    let publish = move |_| {
        spawn_local(async move {
            match api::site_rebuild().await {
                Ok(s) => {
                    status.set(Some(s));
                    start_polling();
                }
                Err(ApiError::Status(409)) => {
                    toast.ok("Ya hay una publicación en curso");
                    start_polling();
                }
                Err(ApiError::Status(503)) => {
                    toast.err("Publicación no configurada en el servidor")
                }
                Err(_) => toast.err("No se pudo iniciar la publicación"),
            }
        });
    };

    let unconfigured =
        move || matches!(status.get(), Some(s) if s.state == SiteBuildState::Unconfigured);
    let busy = move || active.get();

    view! {
        <section class="rounded-2xl border border-white/10 bg-white/4 p-5">
            <div class="flex flex-wrap items-center justify-between gap-4">
                <div class="space-y-1">
                    <h2 class="text-sm/6 font-semibold text-white">"Publicar al sitio"</h2>
                    <p class="text-xs/5 text-zinc-400">
                        "El sitio público es estático; publica para reconstruirlo con tus cambios (~15s)."
                    </p>
                </div>
                <Button
                    disabled=Signal::derive(move || busy() || unconfigured())
                    on:click=publish
                >
                    {move || if busy() { "Publicando…" } else { "Publicar al sitio" }}
                </Button>
            </div>

            {move || (busy() || matches!(status.get(), Some(s) if s.state != SiteBuildState::Idle))
                .then(|| {
                    let s = status.get().unwrap_or_else(|| SiteBuildStatus::bare(SiteBuildState::Idle));
                    let st = s.state;
                    let bar = if st == SiteBuildState::Failed { "bg-red-500" } else { "bg-white" };
                    view! {
                        <div class="mt-4 space-y-2">
                            <div class="flex items-center justify-between text-xs/5">
                                <span class=if st == SiteBuildState::Failed {
                                    "text-red-400"
                                } else {
                                    "text-zinc-300"
                                }>{label(st)}</span>
                                <span class="tabular-nums text-zinc-500">
                                    {move || if busy() { format!("{}s", elapsed.get()) } else { String::new() }}
                                </span>
                            </div>
                            <div class="h-1.5 w-full overflow-hidden rounded-full bg-white/10">
                                <div
                                    class=format!("h-full rounded-full transition-all duration-500 ease-fluid {bar}")
                                    style:width=move || format!("{}%", pct(st))
                                ></div>
                            </div>
                            {s.run_url.map(|url| view! {
                                <a href=url target="_blank" rel="noreferrer"
                                    class="inline-block text-xs/5 text-zinc-500 underline-offset-2 hover:text-zinc-300 hover:underline">
                                    "ver registro de la construcción"
                                </a>
                            })}
                        </div>
                    }
                })}

            {move || unconfigured().then(|| view! {
                <p class="mt-3 text-xs/5 text-zinc-500">
                    "Falta configurar el disparador en el servidor (GITHUB_DISPATCH_TOKEN)."
                </p>
            })}
        </section>
    }
}
