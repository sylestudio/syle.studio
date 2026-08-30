import { defineConfig } from "astro/config";

// Static output: pages are built from the API at build time and served
// behind the Cloudflare CDN. In production, /media is fronted by Cloudflare
// and routed to the API; in dev, Vite proxies it to the local API process so
// `<img src="/media/...">` resolves without absolute URLs in stored paths.
export default defineConfig({
  output: "static",
  site: "https://syle.studio",
  build: {
    // The site CSS is intentionally small. Inlining it removes two blocking
    // round trips on mobile and lets the first headline paint immediately.
    inlineStylesheets: "always",
  },
  vite: {
    server: {
      proxy: {
        "/media": {
          target: "http://127.0.0.1:8080",
          changeOrigin: true,
        },
      },
      watch: {
        usePolling: true,
        interval: 300,
      },
    },
  },
});
