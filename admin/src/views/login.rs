use crate::api;
use crate::ui::{Button, Card, ErrorText, Field, INPUT};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use syle_types::LoginRequest;

#[component]
pub fn Login() -> impl IntoView {
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let pending = RwSignal::new(false);

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
        let nav = use_navigate();
        wasm_bindgen_futures::spawn_local(async move {
            match api::login(&req).await {
                Ok(_) => nav("/", Default::default()),
                Err(_) => error.set(Some("Credenciales inválidas".into())),
            }
            pending.set(false);
        });
    };

    view! {
        <div class="flex min-h-full items-center justify-center p-6 bg-zinc-50">
            <div class="w-full max-w-sm">
                <Card>
                    <form on:submit=submit class="space-y-6">
                        <h1 class="text-xl/8 font-semibold text-zinc-950">
                            "syle.studio · CRM"
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
                </Card>
            </div>
        </div>
    }
}
