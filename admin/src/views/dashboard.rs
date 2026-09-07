use crate::api::{self, ApiError};
use crate::ui::Heading;
use crate::views::site_publish::SitePublish;
use icondata::{
    HiArrowTopRightOnSquareOutlineLg, HiChevronRightOutlineLg, HiDocumentTextOutlineLg,
    HiPhotoOutlineLg,
};
use leptos::prelude::*;
use leptos_icons::Icon as HeroIcon;
use leptos_router::hooks::use_navigate;
use syle_types::PostStatus;
use wasm_bindgen_futures::spawn_local;

type ContentSummary = RwSignal<Option<(usize, usize)>>;

#[component]
fn QuickAccess(
    #[prop(into)] href: String,
    #[prop(into)] title: String,
    #[prop(into)] description: String,
    icon: icondata::Icon,
    #[prop(into)] icon_class: String,
    summary: ContentSummary,
) -> impl IntoView {
    let icon_class = format!(
        "flex size-10 items-center justify-center rounded-xl border {icon_class}"
    );
    view! {
        <a
            href=href
            class="group reveal-item flex min-h-44 flex-col justify-between rounded-2xl \
                border border-white/10 bg-white/4 p-6 transition duration-300 \
                ease-fluid hover:-translate-y-0.5 hover:border-white/20 hover:bg-white/6 \
                motion-reduce:transition-none motion-reduce:hover:translate-y-0"
        >
            <div class="space-y-4">
                <div class="flex items-center justify-between">
                    <span class=icon_class aria-hidden="true">
                        <HeroIcon icon=icon width="1.25rem" height="1.25rem" />
                    </span>
                    <span class="flex size-4 text-zinc-500 \
                        transition-transform group-hover:translate-x-0.5 \
                        group-hover:text-white motion-reduce:transition-none" aria-hidden="true">
                        <HeroIcon icon=HiChevronRightOutlineLg width="1rem" height="1rem" />
                    </span>
                </div>
                <div>
                    <h2 class="text-base/7 font-semibold text-white">{title}</h2>
                    <p class="mt-1 text-sm/6 text-zinc-400">{description}</p>
                </div>
            </div>
            <p class="mt-5 text-xs/5 font-medium text-zinc-500">
                {move || match summary.get() {
                    Some((total, visible)) => format!("{total} en total · {visible} visibles"),
                    None => "Cargando resumen…".to_string(),
                }}
            </p>
        </a>
    }
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let galleries: ContentSummary = RwSignal::new(None);
    let projects: ContentSummary = RwSignal::new(None);
    let posts: ContentSummary = RwSignal::new(None);
    let auth_failed = RwSignal::new(false);
    let navigate = use_navigate();

    Effect::new(move |_| {
        spawn_local(async move {
            match api::list_galleries().await {
                Ok(items) => galleries.set(Some((
                    items.len(),
                    items.iter().filter(|item| item.published).count(),
                ))),
                Err(ApiError::Unauthorized) => auth_failed.set(true),
                Err(_) => {}
            }
            match api::list_projects().await {
                Ok(items) => projects.set(Some((
                    items.len(),
                    items.iter().filter(|item| item.published).count(),
                ))),
                Err(ApiError::Unauthorized) => auth_failed.set(true),
                Err(_) => {}
            }
            match api::list_posts().await {
                Ok(items) => posts.set(Some((
                    items.len(),
                    items
                        .iter()
                        .filter(|item| item.status == PostStatus::Published)
                        .count(),
                ))),
                Err(ApiError::Unauthorized) => auth_failed.set(true),
                Err(_) => {}
            }
        });
    });
    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if auth_failed.get() {
                navigate("/login", Default::default());
            }
        }
    });

    view! {
        <div class="space-y-12">
            <div class="space-y-1">
                <Heading text="Portafolio" />
                <p class="text-sm/6 text-zinc-400">
                    "Resumen del contenido y accesos rápidos del estudio."
                </p>
            </div>

            <section class="space-y-4">
                <h2 class="text-xs/6 font-medium tracking-wide text-zinc-500 uppercase">
                    "Contenido"
                </h2>
                <div class="grid gap-4 md:grid-cols-3">
                    <QuickAccess
                        href="/galleries"
                        title="Galerías"
                        description="Administra colecciones y fotografías."
                        icon=HiPhotoOutlineLg
                        icon_class="border-fuchsia-400/20 bg-fuchsia-400/10 text-fuchsia-300"
                        summary=galleries
                    />
                    <QuickAccess
                        href="/projects"
                        title="Proyectos"
                        description="Administra enlaces y portadas externas."
                        icon=HiArrowTopRightOnSquareOutlineLg
                        icon_class="border-sky-400/20 bg-sky-400/10 text-sky-300"
                        summary=projects
                    />
                    <QuickAccess
                        href="/posts"
                        title="Blog"
                        description="Escribe, revisa y publica entradas."
                        icon=HiDocumentTextOutlineLg
                        icon_class="border-amber-400/20 bg-amber-400/10 text-amber-300"
                        summary=posts
                    />
                </div>
            </section>

            <SitePublish />
        </div>
    }
}
