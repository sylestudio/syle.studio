import type { APIRoute } from "astro";
import { buildSitemapIndex, xmlResponse } from "../lib/seo";

export const GET: APIRoute = ({ site }) => {
  const base = site ?? new URL("https://syle.studio");
  return xmlResponse(buildSitemapIndex(base, ["/sitemap-pages.xml"]));
};
