//! Pre-auth recovery: redeem a single-use code to regain access when passkeys
//! are unavailable. Mirrors the login screen's shell.

use crate::ui::{Button, Card, ErrorText, Field, INPUT};
use crate::webauthn;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

#[component]
pub fn Recovery() -> impl IntoView {
    let (email, set_email) = signal(String::new());
    let (code, set_code) = signal(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let pending = RwSignal::new(false);
    let navigate = use_navigate();

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get() {
            return;
        }
        pending.set(true);
        error.set(None);
        let navigate = navigate.clone();
        let (em, cd) = (email.get(), code.get());
        wasm_bindgen_futures::spawn_local(async move {
            match webauthn::redeem(em, cd).await {
                // Land on Seguridad so the operator re-enrolls a passkey and
                // regenerates codes right away (a redeemed code is now spent).
                Ok(_) => navigate("/security", Default::default()),
                Err(_) => error.set(Some("Código inválido o ya usado".into())),
            }
            pending.set(false);
        });
    };

    view! {
        <div class="flex min-h-svh items-center justify-center bg-zinc-950 p-6">
            <div class="w-full max-w-sm">
                <div class="mb-8 text-center">
                    <span class="text-2xl font-semibold tracking-tight text-white">
                        "syle"<span class="text-zinc-500">".studio"</span>
                    </span>
                    <p class="mt-1 text-sm/6 text-zinc-500">"estudio · CRM"</p>
                </div>
                <Card>
                    <form on:submit=submit class="space-y-6">
                        <h1 class="text-base/7 font-semibold text-white">
                            "Código de recuperación"
                        </h1>
                        <Field label="Correo">
                            <input
                                class=INPUT
                                type="email"
                                autocomplete="username"
                                prop:value=email
                                on:input=move |e| set_email.set(event_target_value(&e))
                            />
                        </Field>
                        <Field label="Código">
                            <input
                                class=INPUT
                                type="text"
                                autocomplete="one-time-code"
                                placeholder="XXXXX-XXXXX"
                                prop:value=code
                                on:input=move |e| set_code.set(event_target_value(&e))
                            />
                        </Field>
                        {move || error.get().map(|m| view! { <ErrorText msg=m /> })}
                        <Button disabled=Signal::derive(move || pending.get())>
                            {move || if pending.get() { "Entrando…" } else { "Entrar" }}
                        </Button>
                        <a
                            href="/login"
                            class="block text-center text-sm/6 text-zinc-400 hover:text-white transition-colors"
                        >
                            "Volver a iniciar sesión"
                        </a>
                    </form>
                </Card>
            </div>
        </div>
    }
}
