use crate::api;
use crate::ui::{Button, Card, ErrorText, Field, INPUT};
use crate::webauthn::{self, PasskeyError};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use syle_types::LoginRequest;

#[component]
pub fn Login() -> impl IntoView {
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let pending = RwSignal::new(false);
    // Must be obtained in the component body (Router context lives on the
    // reactive owner here, not inside the async task below).
    let navigate = use_navigate();
    let nav_passkey = navigate.clone();

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get() {
            return;
        }
        let req = LoginRequest {
            email: email.get(),
            password: password.get(),
        };
        pending.set(true);
        error.set(None);
        let navigate = navigate.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match api::login(&req).await {
                Ok(_) => navigate("/", Default::default()),
                Err(_) => error.set(Some("Credenciales inválidas".into())),
            }
            pending.set(false);
        });
    };

    // Email-first passkey login: type the email, then tap the authenticator.
    let passkey = move |_| {
        if pending.get() {
            return;
        }
        let em = email.get();
        if em.is_empty() {
            error.set(Some("Escribe tu correo para usar tu passkey".into()));
            return;
        }
        pending.set(true);
        error.set(None);
        let navigate = nav_passkey.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match webauthn::login(em).await {
                Ok(_) => navigate("/", Default::default()),
                // User dismissed the prompt — let them try again, no error.
                Err(PasskeyError::Cancelled) => pending.set(false),
                Err(PasskeyError::Api(_)) => {
                    error.set(Some("No se pudo iniciar con tu passkey".into()));
                    pending.set(false);
                }
            }
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
                            "Inicia sesión"
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
                        <Field label="Contraseña">
                            <input
                                class=INPUT
                                type="password"
                                autocomplete="current-password"
                                prop:value=password
                                on:input=move |e| set_password.set(event_target_value(&e))
                            />
                        </Field>
                        {move || error.get().map(|m| view! { <ErrorText msg=m /> })}
                        <Button disabled=Signal::derive(move || pending.get())>
                            {move || if pending.get() { "Entrando…" } else { "Entrar" }}
                        </Button>
                    </form>
                    <div class="mt-6 flex items-center gap-3">
                        <span class="h-px flex-1 bg-white/10"></span>
                        <span class="text-xs/5 text-zinc-500">"o"</span>
                        <span class="h-px flex-1 bg-white/10"></span>
                    </div>
                    <button
                        type="button"
                        class="mt-6 inline-flex w-full items-center justify-center rounded-lg \
                            border border-white/15 px-3.5 py-2.5 text-sm/6 font-semibold text-white \
                            hover:bg-white/5 transition duration-200 ease-fluid active:scale-[0.98] \
                            disabled:opacity-50 disabled:pointer-events-none \
                            motion-reduce:transition-none motion-reduce:active:scale-100"
                        on:click=passkey
                        prop:disabled=move || pending.get()
                    >
                        "Entrar con passkey"
                    </button>
                    <a
                        href="/recovery"
                        class="mt-4 block text-center text-sm/6 text-zinc-400 hover:text-white transition-colors"
                    >
                        "¿Perdiste el acceso? Usa un código de recuperación"
                    </a>
                </Card>
            </div>
        </div>
    }
}
