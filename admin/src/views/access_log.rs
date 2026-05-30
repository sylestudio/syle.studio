//! Read-only access log: recent authentication events (logins by password /
//! passkey / recovery, logout, and key-management actions). Every value comes
//! from the audit trail and may be attacker-influenced (email/IP/User-Agent on
//! a failed pre-auth login), so all cells render on Leptos's escaped text path —
//! never `inner_html` — and the IP is shown as best-effort, untrusted context.

use crate::api;
use crate::ui::{Badge, Card, Heading};
use leptos::prelude::*;
use syle_types::{AccessAction, AccessLogEntry, AccessMethod, AccessOutcome};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

// Full literal class strings so Tailwind v4 `@source` scans them (no `format!`).
const TABLE: &str = "w-full text-left text-sm/6";
const TH: &str = "px-3 py-2 text-xs/5 font-medium uppercase tracking-wide text-zinc-500";
const TD: &str = "px-3 py-3 align-top text-zinc-300";
const TD_TIME: &str = "px-3 py-3 align-top whitespace-nowrap text-zinc-400 tabular-nums";
const ROW: &str = "border-t border-white/5";
const UA: &str = "block max-w-[16rem] truncate text-zinc-400";
const REFRESH: &str = "rounded-lg px-2.5 py-1 text-xs/5 font-medium text-zinc-300 \
    hover:bg-white/10 hover:text-white transition-colors";

/// Localized timestamp from Unix seconds (browser locale formatting).
fn fmt_time(secs: i64) -> String {
    let d = js_sys::Date::new(&JsValue::from_f64(secs as f64 * 1000.0));
    String::from(d.to_locale_string("es-MX", &JsValue::UNDEFINED))
}

/// Human label for the event kind.
fn event_label(action: AccessAction, method: Option<AccessMethod>) -> &'static str {
    match action {
        AccessAction::Login => match method {
            Some(AccessMethod::Password) => "Inicio con contraseña",
            Some(AccessMethod::Passkey) => "Inicio con llave",
            Some(AccessMethod::Recovery) => "Inicio con código",
            None => "Inicio de sesión",
        },
        AccessAction::Logout => "Cierre de sesión",
        AccessAction::PasskeyEnroll => "Llave registrada",
        AccessAction::PasskeyRevoke => "Llave revocada",
        AccessAction::RecoveryGenerate => "Códigos generados",
    }
}

#[component]
pub fn AccessLog() -> impl IntoView {
    let entries = RwSignal::new(Vec::<AccessLogEntry>::new());

    let reload = move || {
        spawn_local(async move {
            if let Ok(list) = api::access_log().await {
                entries.set(list);
            }
        });
    };
    Effect::new(move |_| reload());

    view! {
        <div class="space-y-8">
            <Heading text="Accesos" />

            <Card>
                <div class="flex items-start justify-between gap-4">
                    <div>
                        <h2 class="text-base/7 font-semibold text-white">"Actividad reciente"</h2>
                        <p class="mt-1 text-sm/6 text-zinc-400">
                            "Inicios de sesión, intentos fallidos y cambios en tus llaves. \
                             La IP es orientativa (la reporta el proxy) y no es prueba de identidad."
                        </p>
                    </div>
                    <button class=REFRESH on:click=move |_| reload()>"Refrescar"</button>
                </div>

                <div class="mt-6 overflow-x-auto">
                    <table class=TABLE>
                        <thead>
                            <tr>
                                <th class=TH>"Fecha"</th>
                                <th class=TH>"Evento"</th>
                                <th class=TH>"Cuenta"</th>
                                <th class=TH>"IP"</th>
                                <th class=TH>"Navegador"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                let list = entries.get();
                                if list.is_empty() {
                                    return view! {
                                        <tr>
                                            <td class=TD colspan="5">
                                                <span class="text-zinc-500">
                                                    "Aún no hay accesos registrados."
                                                </span>
                                            </td>
                                        </tr>
                                    }
                                    .into_any();
                                }
                                list.into_iter()
                                    .map(|e| {
                                        let failed = e.outcome == AccessOutcome::Failure;
                                        let ua = e.user_agent.clone().unwrap_or_default();
                                        let ua_title = ua.clone();
                                        view! {
                                            <tr class=ROW>
                                                <td class=TD_TIME>{fmt_time(e.at)}</td>
                                                <td class=TD>
                                                    <div class="flex flex-wrap items-center gap-2">
                                                        <span class="text-white">
                                                            {event_label(e.action, e.method)}
                                                        </span>
                                                        {if failed {
                                                            view! { <Badge tone="red">"Fallido"</Badge> }
                                                                .into_any()
                                                        } else if e.action == AccessAction::Login {
                                                            view! { <Badge tone="green">"OK"</Badge> }
                                                                .into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                    </div>
                                                </td>
                                                <td class=TD>
                                                    {e.email.clone().unwrap_or_else(|| "—".into())}
                                                </td>
                                                <td class=TD>
                                                    {e.ip.clone().unwrap_or_else(|| "—".into())}
                                                </td>
                                                <td class=TD>
                                                    <span class=UA title=ua_title>{ua}</span>
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }}
                        </tbody>
                    </table>
                </div>
            </Card>
        </div>
    }
}
