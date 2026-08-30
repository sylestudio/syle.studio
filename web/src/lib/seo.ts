const escapeXml = (value: string) =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");

export const normalizeRoute = (route: string) => {
  const [, pathname = route, suffix = ""] = route.match(/^([^?#]*)(.*)$/) ?? [];
  return pathname === "/" || pathname.endsWith("/")
    ? route
    : `${pathname}/${suffix}`;
};

export const buildUrlSet = (base: URL, routes: Iterable<string>) => {
  const urls = [...new Set([...routes].map(normalizeRoute))]
    .sort()
    .map((route) => `  <url><loc>${escapeXml(new URL(route, base).href)}</loc></url>`)
    .join("\n");

  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    urls,
    "</urlset>",
    "",
  ].join("\n");
};

export const buildSitemapIndex = (base: URL, sitemapPaths: Iterable<string>) => {
  const sitemaps = [...new Set(sitemapPaths)]
    .sort()
    .map((route) => `  <sitemap><loc>${escapeXml(new URL(route, base).href)}</loc></sitemap>`)
    .join("\n");

  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    sitemaps,
    "</sitemapindex>",
    "",
  ].join("\n");
};

export const xmlResponse = (body: string) =>
  new Response(body, {
    headers: { "Content-Type": "application/xml; charset=utf-8" },
  });
