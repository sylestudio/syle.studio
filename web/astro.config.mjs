import { defineConfig } from "astro/config";

// Static output: pages are built from the API at build time and served
// behind the Cloudflare CDN.
export default defineConfig({
  output: "static",
  site: "https://syle.studio",
});
