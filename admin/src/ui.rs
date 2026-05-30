//! Design-system primitives ported from Catalyst (Tailwind Plus), tuned for
//! the CRM's dark "studio" shell. Visual classes are kept faithful; Headless-UI
//! `data-*` state is replaced with native `hover:`/`focus:` (no Headless UI in
//! Leptos). Color/variant classes are enumerated as literal `const` strings so
//! the Tailwind v4 `@source` scanner sees them (no `format!`-built classes).

use leptos::prelude::*;

const BTN: &str = "relative inline-flex items-center justify-center gap-x-2 \
    rounded-lg border border-transparent px-3.5 py-2.5 sm:px-3 sm:py-1.5 \
    text-base/6 sm:text-sm/6 font-semibold \
    focus:outline-2 focus:outline-offset-2 focus:outline-blue-500 \
    disabled:opacity-50 disabled:pointer-events-none \
    transition duration-200 ease-fluid active:scale-[0.98] \
    motion-reduce:transition-none motion-reduce:active:scale-100";

#[component]
pub fn Button(
    #[prop(optional, into)] kind: String,
    #[prop(optional)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let palette = match kind.as_str() {
        "outline" => "border-white/15 text-white hover:bg-white/5",
        "plain" => "text-zinc-300 hover:bg-white/10 hover:text-white",
        "danger" => "bg-red-600 text-white shadow-sm hover:bg-red-500",
        _ => "bg-white text-zinc-950 shadow-sm hover:bg-zinc-200",
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
        <h1 class="text-2xl/8 sm:text-xl/8 font-semibold tracking-tight text-white">
            {text}
        </h1>
    }
}

#[component]
pub fn Field(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <div class="space-y-1.5">
            <label class="text-sm/6 font-medium text-zinc-300">{label}</label>
            {children()}
        </div>
    }
}

/// Class string for `<input>`/`<textarea>` matching Catalyst's dark input look.
pub const INPUT: &str = "block w-full rounded-lg border border-white/10 \
    bg-white/5 px-3 py-1.5 text-base/6 sm:text-sm/6 text-white shadow-sm \
    placeholder:text-zinc-500 focus:outline-2 focus:-outline-offset-2 \
    focus:outline-blue-500";

#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! {
        <div class="rounded-2xl border border-white/10 bg-white/2.5 p-6">
            {children()}
        </div>
    }
}

/// Tactile-depth media tile: a nested-bezel enclosure (outer "shell" + inner
/// "core") that makes gallery/post cards read as physical objects instead of
/// flat fills. Lifts and brightens its hairline on hover with the fluid easing;
/// motion is suppressed under `prefers-reduced-motion`. The caller supplies the
/// inner content (cover + meta); concentric radii are handled here.
#[component]
pub fn Tile(children: Children) -> impl IntoView {
    view! {
        <div class="rounded-2xl border border-white/10 bg-white/4 p-1.5 \
            transition duration-300 ease-fluid \
            hover:-translate-y-0.5 hover:border-white/20 hover:bg-white/6 \
            motion-reduce:transition-none motion-reduce:hover:translate-y-0">
            <div class="bezel-core overflow-hidden rounded-xl bg-zinc-900">
                {children()}
            </div>
        </div>
    }
}

/// Shimmering placeholder block (use while a cover/image loads). Pass extra
/// classes for sizing/shape, e.g. `class="aspect-square rounded-xl"`.
#[component]
pub fn Skeleton(#[prop(optional, into)] class: String) -> impl IntoView {
    view! { <div class=format!("skeleton {class}")></div> }
}

#[component]
pub fn ErrorText(#[prop(into)] msg: String) -> impl IntoView {
    view! { <p class="text-sm/6 text-red-400">{msg}</p> }
}

const BADGE: &str = "inline-flex items-center gap-x-1.5 rounded-md px-2 py-0.5 \
    text-xs/5 font-medium";

/// Small status pill. `tone`: "green" | "amber" | "red" | "zinc" (default).
#[component]
pub fn Badge(#[prop(optional, into)] tone: String, children: Children) -> impl IntoView {
    let colors = match tone.as_str() {
        "green" => "bg-green-500/15 text-green-400",
        "amber" => "bg-amber-400/15 text-amber-400",
        "red" => "bg-red-500/15 text-red-400",
        _ => "bg-white/5 text-zinc-400",
    };
    view! { <span class=format!("{BADGE} {colors}")>{children()}</span> }
}

/// Right-anchored slide-over panel. Mounts only while `open` is true; the
/// backdrop and the ✕ button both close it (no global Esc — keeps it
/// dependency-free; Headless-UI's Dialog is not portable to Leptos).
#[component]
pub fn SlideOver(
    open: RwSignal<bool>,
    #[prop(into)] title: String,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        {move || open.get().then(|| view! {
            <div class="fixed inset-0 z-40">
                <div
                    class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                    on:click=move |_| open.set(false)
                ></div>
                <div class="absolute inset-y-0 right-0 flex w-full max-w-md flex-col \
                    border-l border-white/10 bg-zinc-900 p-6 shadow-2xl">
                    <div class="mb-6 flex items-center justify-between">
                        <h2 class="text-lg/7 font-semibold text-white">
                            {title.clone()}
                        </h2>
                        <button
                            class="rounded-lg p-1 text-zinc-400 hover:bg-white/10 \
                                hover:text-white transition-colors"
                            on:click=move |_| open.set(false)
                            aria-label="Cerrar"
                        >
                            "✕"
                        </button>
                    </div>
                    <div class="flex-1 overflow-y-auto">{children()}</div>
                </div>
            </div>
        })}
    }
}

/// Centered modal dialog. The backdrop covers the whole page so it blocks
/// every other action until the user resolves it (used for destructive
/// confirmations). Backdrop click cancels.
#[component]
pub fn Modal(
    open: RwSignal<bool>,
    #[prop(into)] title: String,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        {move || open.get().then(|| view! {
            <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
                <div
                    class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                    on:click=move |_| open.set(false)
                ></div>
                <div class="relative w-full max-w-md rounded-2xl border border-white/10 \
                    bg-zinc-900 p-6 shadow-2xl">
                    <h2 class="text-base/7 font-semibold text-white">
                        {title.clone()}
                    </h2>
                    <div class="mt-3">{children()}</div>
                </div>
            </div>
        })}
    }
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
                    "rounded-lg bg-white px-4 py-2 text-sm/6 font-medium text-zinc-950 shadow-lg"
                } else {
                    "rounded-lg bg-red-600 px-4 py-2 text-sm/6 font-medium text-white shadow-lg"
                };
                view! { <div class=cls>{toast.msg}</div> }
            }).collect_view()}
        </div>
    }
}
