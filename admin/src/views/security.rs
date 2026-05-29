//! Account security: enroll/revoke passkeys and (re)generate recovery codes.
//! Lives inside the authenticated shell.

use crate::api;
use crate::ui::{use_toaster, Badge, Card, Heading, Modal};
use crate::webauthn::{self, PasskeyError};
use leptos::prelude::*;
use syle_types::{CredentialInfo, RenameCredential};
use wasm_bindgen_futures::spawn_local;

// Literal class strings (Tailwind v4 `@source` scans for these; no `format!`).
const BTN_PRIMARY: &str = "inline-flex items-center justify-center rounded-lg px-3.5 py-2 \
    text-sm/6 font-semibold bg-white text-zinc-950 shadow-sm hover:bg-zinc-200 \
    transition duration-200 ease-fluid active:scale-[0.98] disabled:opacity-50 \
    disabled:pointer-events-none motion-reduce:transition-none motion-reduce:active:scale-100";
const BTN_REVOKE: &str = "rounded-lg px-2.5 py-1 text-xs/5 font-medium text-red-400 \
    hover:bg-red-500/10 transition-colors";
const NAME_INPUT: &str = "min-w-0 rounded-lg border border-white/10 bg-white/5 px-2.5 py-1 \
    text-sm/6 text-white placeholder:text-zinc-500 focus:outline-2 focus:-outline-offset-2 \
    focus:outline-blue-500";

#[component]
pub fn Security() -> impl IntoView {
    let t = use_toaster();
    let creds = RwSignal::new(Vec::<CredentialInfo>::new());
    let busy = RwSignal::new(false);

    let reload = move || {
        spawn_local(async move {
            if let Ok(list) = api::webauthn_credentials().await {
                creds.set(list);
            }
        });
    };
    Effect::new(move |_| reload());

    let add = move |_| {
        if busy.get() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match webauthn::enroll().await {
                Ok(_) => {
                    t.ok("Llave registrada");
                    reload();
                }
                // User dismissed the OS prompt — stay quiet.
                Err(PasskeyError::Cancelled) => {}
                Err(PasskeyError::Api(_)) => t.err("No se pudo registrar la llave"),
            }
            busy.set(false);
        });
    };

    let revoke = move |id: String| {
        spawn_local(async move {
            match api::webauthn_delete_credential(&id).await {
                Ok(()) => {
                    t.ok("Llave revocada");
                    reload();
                }
                Err(api::ApiError::Status(400)) => {
                    t.err("Es tu último factor: añade otra llave o un respaldo antes")
                }
                Err(_) => t.err("No se pudo revocar"),
            }
        });
    };

    let rename = move |id: String, name: String| {
        spawn_local(async move {
            match api::webauthn_rename_credential(&id, &RenameCredential { name }).await {
                Ok(_) => {
                    t.ok("Nombre actualizado");
                    reload();
                }
                Err(_) => t.err("No se pudo renombrar"),
            }
        });
    };

    // Recovery codes are shown exactly once, in a modal.
    let codes = RwSignal::new(Vec::<String>::new());
    let show_codes = RwSignal::new(false);
    let generate = move |_| {
        spawn_local(async move {
            match api::recovery_generate().await {
                Ok(rc) => {
                    codes.set(rc.codes);
                    show_codes.set(true);
                }
                Err(_) => t.err("No se pudieron generar los códigos"),
            }
        });
    };

    view! {
        <div class="space-y-8">
            <Heading text="Seguridad" />

            <Card>
                <div class="flex items-start justify-between gap-4">
                    <div>
                        <h2 class="text-base/7 font-semibold text-white">"Llaves de acceso"</h2>
                        <p class="mt-1 text-sm/6 text-zinc-400">
                            "Inicia sesión con Touch ID, Windows Hello o una llave de seguridad. \
                             El servidor solo guarda la clave pública."
                        </p>
                    </div>
                    <button class=BTN_PRIMARY on:click=add prop:disabled=move || busy.get()>
                        {move || if busy.get() { "Registrando…" } else { "Añadir llave" }}
                    </button>
                </div>

                <ul class="mt-6 divide-y divide-white/5">
                    {move || {
                        let list = creds.get();
                        if list.is_empty() {
                            return view! {
                                <li class="py-3 text-sm/6 text-zinc-500">
                                    "Aún no hay llaves registradas."
                                </li>
                            }
                            .into_any();
                        }
                        list.into_iter()
                            .map(|c| {
                                let id = c.id.to_string();
                                let id_r = id.clone();
                                let used = c.last_used_at.is_some();
                                view! {
                                    <li class="flex items-center justify-between gap-3 py-3">
                                        <div class="flex min-w-0 flex-1 items-center gap-3">
                                            <input
                                                class=NAME_INPUT
                                                prop:value=c.name.clone()
                                                placeholder="Nombre (Enter para guardar)"
                                                on:keydown=move |e: leptos::ev::KeyboardEvent| {
                                                    if e.key() == "Enter" {
                                                        rename(id_r.clone(), event_target_value(&e));
                                                    }
                                                }
                                            />
                                            {if used {
                                                view! { <Badge tone="green">"Activa"</Badge> }.into_any()
                                            } else {
                                                view! { <Badge tone="amber">"Sin usar"</Badge> }.into_any()
                                            }}
                                        </div>
                                        <button
                                            class=BTN_REVOKE
                                            on:click=move |_| revoke(id.clone())
                                        >
                                            "Revocar"
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </ul>
            </Card>

            <Card>
                <div class="flex items-start justify-between gap-4">
                    <div>
                        <h2 class="text-base/7 font-semibold text-white">"Códigos de recuperación"</h2>
                        <p class="mt-1 text-sm/6 text-zinc-400">
                            "Diez códigos de un solo uso para entrar si pierdes tus llaves. \
                             Generarlos invalida cualquier conjunto anterior."
                        </p>
                    </div>
                    <button class=BTN_PRIMARY on:click=generate>"Generar"</button>
                </div>
            </Card>

            <Modal open=show_codes title="Guarda estos códigos">
                <p class="text-sm/6 text-zinc-400">
                    "Se muestran una sola vez. Guárdalos en un lugar seguro: cada uno sirve \
                     para un único inicio de sesión."
                </p>
                <ul class="mt-4 grid grid-cols-2 gap-2 font-mono text-sm text-white">
                    {move || {
                        codes
                            .get()
                            .into_iter()
                            .map(|c| view! {
                                <li class="rounded-lg bg-white/5 px-3 py-1.5 text-center">{c}</li>
                            })
                            .collect_view()
                    }}
                </ul>
            </Modal>
        </div>
    }
}
