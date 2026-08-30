import type { APIRoute } from "astro";
import { listGalleries, listPosts } from "../lib/api";
import { projectHref } from "../lib/proyectos";

const projectRoutes = [
  "/proyectos/catedral/",
  "/proyectos/dango/",
  "/proyectos/insomnio/",
  "/proyectos/laobsesion/",
  "/proyectos/rupture/",
  "/proyectos/syle/",
];

const escapeXml = (value: string) =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");

const normalizeRoute = (route: string) =>
  route === "/" || route.endsWith("/") ? route : `${route}/`;

export const GET: APIRoute = async ({ site }) => {
  const base = site ?? new URL("https://syle.studio");
  const [galleries, posts] = await Promise.all([listGalleries(), listPosts()]);
  const routes = new Set<string>(["/", "/blog/", ...projectRoutes]);

  galleries.forEach((gallery) => routes.add(normalizeRoute(projectHref(gallery.slug))));
  posts.forEach((post) => routes.add(`/blog/${post.slug}/`));

  const urls = [...routes]
    .sort()
    .map((route) => `  <url><loc>${escapeXml(new URL(route, base).href)}</loc></url>`)
    .join("\n");
  const body = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    urls,
    "</urlset>",
    "",
  ].join("\n");

  return new Response(body, {
    headers: { "Content-Type": "application/xml; charset=utf-8" },
  });
};
