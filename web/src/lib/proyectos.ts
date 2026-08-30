// Which project slugs have a hand-built "proyecto completo" page under
// src/pages/proyectos/. Detected at build time so the home grid and the
// gallery navigator can deep-link straight to that bespoke page instead of the
// generic /galleries/[slug] view. Slugs without one fall back to /galleries.
const bespokeSlugs = new Set(
  Object.keys(import.meta.glob("../pages/proyectos/*.astro")).map((path) =>
    path.replace(/^.*\/proyectos\/(.+)\.astro$/, "$1"),
  ),
);

/** True when `src/pages/proyectos/<slug>.astro` exists. */
export const hasBespokePage = (slug: string): boolean => bespokeSlugs.has(slug);

/** Canonical URL for a project: its bespoke page when there is one. */
export const projectHref = (slug: string): string =>
  bespokeSlugs.has(slug) ? `/proyectos/${slug}/` : `/galleries/${slug}/`;
