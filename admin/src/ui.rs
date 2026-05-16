//! Design-system primitives ported from Catalyst (Tailwind Plus).
//! Visual classes are kept faithful; Headless-UI `data-*` state is replaced
//! with native `hover:`/`focus:` since Leptos has no Headless UI.

use leptos::prelude::*;

const BTN: &str = "relative inline-flex items-center justify-center gap-x-2 \
    rounded-lg border border-transparent px-3.5 py-2.5 sm:px-3 sm:py-1.5 \
    text-base/6 sm:text-sm/6 font-semibold shadow-sm \
    focus:outline-2 focus:outline-offset-2 focus:outline-blue-500 \
    disabled:opacity-50 disabled:pointer-events-none transition-colors";

#[component]
pub fn Button(
    #[prop(optional, into)] kind: String,
    #[prop(optional)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let palette = match kind.as_str() {
        "outline" => "border-zinc-950/10 text-zinc-950 bg-white hover:bg-zinc-950/2.5",
        "plain" => "text-zinc-950 shadow-none hover:bg-zinc-950/5",
        _ => "bg-zinc-900 text-white hover:bg-zinc-700",
    };
    view! {
        <button class=format!("{BTN} {palette}") disabled=disabled>
            {children()}
        </button>
    }
}

#[component]
pub fn Heading(#[prop(into)] text: String) -> impl IntoView {
    view! {
        <h1 class="text-2xl/8 sm:text-xl/8 font-semibold text-zinc-950">{text}</h1>
    }
}

#[component]
pub fn Field(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <div class="space-y-1.5">
            <label class="text-sm/6 font-medium text-zinc-950">{label}</label>
            {children()}
        </div>
    }
}

/// Class string for `<input>`/`<textarea>` matching Catalyst's input look.
pub const INPUT: &str = "block w-full rounded-lg border border-zinc-950/10 \
    bg-white px-3 py-1.5 text-base/6 sm:text-sm/6 text-zinc-950 shadow-sm \
    placeholder:text-zinc-500 focus:outline-2 focus:-outline-offset-2 \
    focus:outline-blue-500";

#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-zinc-950/10 bg-white p-6 shadow-sm">
            {children()}
        </div>
    }
}

#[component]
pub fn ErrorText(#[prop(into)] msg: String) -> impl IntoView {
    view! { <p class="text-sm/6 text-red-600">{msg}</p> }
}
