import { describe, expect, it } from "vitest";
import { buildSitemapIndex, buildUrlSet, normalizeRoute } from "./seo";

const base = new URL("https://syle.studio");

describe("SEO XML helpers", () => {
  it("normalizes page routes to their single trailing-slash URL", () => {
    expect(normalizeRoute("/")).toBe("/");
    expect(normalizeRoute("/blog")).toBe("/blog/");
    expect(normalizeRoute("/blog/")).toBe("/blog/");
  });

  it("sorts, deduplicates and escapes canonical page URLs", () => {
    const xml = buildUrlSet(base, ["/z", "/a?x=1&y=2", "/z/"]);
    expect(xml).toContain("https://syle.studio/a/?x=1&amp;y=2");
    expect(xml.match(/https:\/\/syle\.studio\/z\//g)).toHaveLength(1);
    expect(xml.indexOf("/a/?")).toBeLessThan(xml.indexOf("/z/"));
  });

  it("builds a sitemap index on the canonical origin", () => {
    expect(buildSitemapIndex(base, ["/sitemap-pages.xml"])).toContain(
      "<loc>https://syle.studio/sitemap-pages.xml</loc>",
    );
  });
});
