import { access, readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const webRoot = fileURLToPath(new URL("..", import.meta.url));
const repoRoot = path.resolve(webRoot, "..");
const dist = path.join(webRoot, "dist");
const canonicalOrigin = "https://syle.studio";

const fail = (message) => {
  throw new Error(`SEO build verification failed: ${message}`);
};
const expect = (condition, message) => {
  if (!condition) fail(message);
};
const read = (relative) => readFile(path.join(dist, relative), "utf8");
const decodeXml = (value) =>
  value
    .replaceAll("&amp;", "&")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&apos;", "'");
const locs = (xml) =>
  [...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map((match) => decodeXml(match[1]));

const listFiles = async (directory, prefix = "") => {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const relative = path.join(prefix, entry.name);
    if (entry.isDirectory()) {
      files.push(...await listFiles(path.join(directory, entry.name), relative));
    } else {
      files.push(relative);
    }
  }
  return files;
};

await access(dist);

const robots = await read("robots.txt");
expect(/^User-agent: \*$/m.test(robots), "robots.txt has no wildcard user-agent");
expect(
  /^Disallow: \/cdn-cgi\/$/m.test(robots),
  "robots.txt does not exclude Cloudflare's internal crawler endpoints",
);
expect(/^Disallow: \/api\/$/m.test(robots), "robots.txt does not exclude API routes");
expect(
  /^Sitemap: https:\/\/syle\.studio\/sitemap-index\.xml$/m.test(robots),
  "robots.txt does not advertise the canonical sitemap index",
);

const home = await read("index.html");
const contactLink = home.match(
  /<a\b[^>]*href="mailto:contacto@sylestudio\.com"[^>]*>([^]*?)<\/a>/,
);
expect(Boolean(contactLink), "home page has no clickable contact email");
expect(
  /contacto@sylestudio\.com/.test(contactLink[1]),
  "home page contact link does not show the email address",
);
expect(
  /<!--email_off-->[^]*href="mailto:contacto@sylestudio\.com"[^]*<!--\/email_off-->/.test(home),
  "home page contact email is not excluded from Cloudflare obfuscation",
);

const sitemapIndex = await read("sitemap-index.xml");
expect(/<sitemapindex\b/.test(sitemapIndex), "sitemap-index.xml is not a sitemap index");
expect(
  JSON.stringify(locs(sitemapIndex)) === JSON.stringify([`${canonicalOrigin}/sitemap-pages.xml`]),
  "sitemap index must reference exactly sitemap-pages.xml",
);

const pageSitemap = await read("sitemap-pages.xml");
expect(/<urlset\b/.test(pageSitemap), "sitemap-pages.xml is not a URL set");
const pageUrls = locs(pageSitemap);
expect(pageUrls.length > 0, "page sitemap is empty");
expect(new Set(pageUrls).size === pageUrls.length, "page sitemap contains duplicate URLs");
expect(
  JSON.stringify([...pageUrls].sort()) === JSON.stringify(pageUrls),
  "page sitemap URLs are not sorted",
);

const sitemapPaths = new Set();
for (const value of pageUrls) {
  const url = new URL(value);
  expect(url.origin === canonicalOrigin, `non-canonical origin in sitemap: ${value}`);
  expect(!url.search && !url.hash, `query or fragment in sitemap URL: ${value}`);
  expect(url.pathname === "/" || url.pathname.endsWith("/"), `page URL lacks trailing slash: ${value}`);
  sitemapPaths.add(url.pathname);

  const htmlPath = url.pathname === "/"
    ? "index.html"
    : path.join(url.pathname.slice(1), "index.html");
  const html = htmlPath === "index.html" ? home : await read(htmlPath);
  const canonicals = [...html.matchAll(/<link\b[^>]*rel="canonical"[^>]*href="([^"]+)"[^>]*>/g)];
  expect(canonicals.length === 1, `${htmlPath} must contain exactly one canonical link`);
  expect(canonicals[0][1] === value, `${htmlPath} canonical differs from its sitemap URL`);
  const metaRobots = html.match(/<meta\b[^>]*name="robots"[^>]*content="([^"]+)"[^>]*>/)?.[1] ?? "";
  expect(/(?:^|,)index(?:,|$)/.test(metaRobots), `${htmlPath} is not explicitly indexable`);
  expect(!/(?:^|,)noindex(?:,|$)/.test(metaRobots), `${htmlPath} is noindex but present in sitemap`);
  expect(html.includes(`property="og:url" content="${value}"`), `${htmlPath} Open Graph URL is not canonical`);
  const jsonLd = html.match(/<script type="application\/ld\+json">([^]*?)<\/script>/)?.[1];
  expect(Boolean(jsonLd), `${htmlPath} has no JSON-LD`);
  JSON.parse(jsonLd);
}

const notFound = await read("404.html");
const notFoundRobots = notFound.match(/<meta\b[^>]*name="robots"[^>]*content="([^"]+)"[^>]*>/)?.[1] ?? "";
expect(notFoundRobots.includes("noindex"), "404 page is not noindex");
expect(notFoundRobots.includes("follow"), "404 page does not allow link following");
expect(!/rel="canonical"/.test(notFound), "404 page must not emit a canonical link");
expect(!/property="og:url"/.test(notFound), "404 page must not emit og:url");
expect(/<h1\b[^>]*>Esta página no existe\.<\/h1>/.test(notFound), "404 page has no useful heading");

const htmlFiles = (await listFiles(dist)).filter((file) => file.endsWith(".html") && file !== "404.html");
const htmlCanonicals = new Set();
for (const htmlFile of htmlFiles) {
  const html = await read(htmlFile);
  const canonical = html.match(/<link\b[^>]*rel="canonical"[^>]*href="([^"]+)"[^>]*>/)?.[1];
  expect(Boolean(canonical), `${htmlFile} has no canonical URL`);
  expect(!htmlCanonicals.has(canonical), `duplicate canonical URL emitted: ${canonical}`);
  htmlCanonicals.add(canonical);

  for (const [, href] of html.matchAll(/\bhref="([^"]+)"/g)) {
    if (!href.startsWith("/")) continue;
    const link = new URL(href, canonicalOrigin);
    const slashVariant = link.pathname === "/" ? "/" : `${link.pathname}/`.replaceAll("//", "/");
    if (sitemapPaths.has(slashVariant) && link.pathname !== slashVariant) {
      fail(`${htmlFile} links to redirecting internal URL ${href}`);
    }
  }
}
expect(
  htmlCanonicals.size === sitemapPaths.size && [...htmlCanonicals].every((url) => pageUrls.includes(url)),
  "sitemap and generated indexable HTML pages are not one-to-one",
);

const publicNginx = await readFile(path.join(repoRoot, "deploy/nginx/syle.studio.conf"), "utf8");
for (const required of [
  "return 301 https://syle.studio$request_uri;",
  "error_page 404 /404.html;",
  "location = /sitemap.xml",
  "return 301 https://syle.studio/sitemap-index.xml$is_args$args;",
  'add_header X-Robots-Tag "noindex, follow" always;',
]) {
  expect(publicNginx.includes(required), `public nginx config is missing: ${required}`);
}

const adminNginx = await readFile(path.join(repoRoot, "deploy/nginx/admin.syle.studio.conf"), "utf8");
expect(
  adminNginx.includes('add_header X-Robots-Tag "noindex, nofollow, noarchive" always;'),
  "admin nginx config does not explicitly exclude the CRM from indexing",
);
expect(adminNginx.includes("location = /robots.txt"), "admin nginx config has no explicit robots.txt");

console.log(`SEO build verified: ${pageUrls.length} canonical pages, sitemap index, robots.txt and real 404 artifact.`);
