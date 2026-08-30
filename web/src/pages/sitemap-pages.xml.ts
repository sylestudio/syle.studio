import type { APIRoute } from "astro";
import { listGalleries, listPosts } from "../lib/api";
import { projectHref } from "../lib/proyectos";
import { buildUrlSet, normalizeRoute, xmlResponse } from "../lib/seo";

const projectRoutes = [
  "/proyectos/catedral/",
  "/proyectos/dango/",
  "/proyectos/insomnio/",
  "/proyectos/laobsesion/",
  "/proyectos/rupture/",
  "/proyectos/syle/",
];

export const GET: APIRoute = async ({ site }) => {
  const base = site ?? new URL("https://syle.studio");
  const [galleries, posts] = await Promise.all([listGalleries(), listPosts()]);
  const routes = new Set<string>(["/", "/blog/", ...projectRoutes]);

  galleries.forEach((gallery) => routes.add(normalizeRoute(projectHref(gallery.slug))));
  posts.forEach((post) => routes.add(`/blog/${post.slug}/`));

  return xmlResponse(buildUrlSet(base, routes));
};
