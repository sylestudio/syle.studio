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

// --- Toasts -----------------------------------------------------------------

#[derive(Clone)]
pub struct Toast {
    pub id: u32,
    pub ok: bool,
    pub msg: String,
}

/// Cheap-to-clone toast queue, provided at the app root via context.
#[derive(Clone, Copy)]
pub struct Toaster {
    items: RwSignal<Vec<Toast>>,
    seq: RwSignal<u32>,
}

impl Toaster {
    pub fn new() -> Self {
        Self {
            items: RwSignal::new(Vec::new()),
            seq: RwSignal::new(0),
        }
    }

    fn push(&self, ok: bool, msg: String) {
        let id = self.seq.get_untracked() + 1;
        self.seq.set(id);
        self.items.update(|v| v.push(Toast { id, ok, msg }));
        let items = self.items;
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(3500).await;
            items.update(|v| v.retain(|t| t.id != id));
        });
    }

    pub fn ok(&self, msg: impl Into<String>) {
        self.push(true, msg.into());
    }

    pub fn err(&self, msg: impl Into<String>) {
        self.push(false, msg.into());
    }
}

impl Default for Toaster {
    fn default() -> Self {
        Self::new()
    }
}

pub fn use_toaster() -> Toaster {
    use_context::<Toaster>().expect("Toaster must be provided at the app root")
}

#[component]
pub fn ToastHost() -> impl IntoView {
    let t = use_toaster();
    view! {
        <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
            {move || t.items.get().into_iter().map(|toast| {
                let cls = if toast.ok {
                    "rounded-lg bg-zinc-900 px-4 py-2 text-sm/6 text-white shadow-lg"
                } else {
                    "rounded-lg bg-red-600 px-4 py-2 text-sm/6 text-white shadow-lg"
                };
                view! { <div class=cls>{toast.msg}</div> }
            }).collect_view()}
        </div>
    }
}
