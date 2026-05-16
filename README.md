# syle.studio

Photography portfolio + tunneled CRM for a studio client. Two independent
sites, one Rust workspace.

| Path     | What                                  | Runtime / build        |
|----------|---------------------------------------|------------------------|
| `types/` | Shared serde DTOs (API ⇄ admin)       | lib, host + wasm       |
| `core/`  | Domain + pure-Rust image pipeline     | lib                    |
| `api/`   | Axum API (public read + CRM write)    | native binary (VPS)    |
| `admin/` | Leptos CSR SPA — `admin.syle.studio`  | wasm32, built w/ trunk |
| `web/`   | Astro public site — `syle.studio`     | Node, SSG/ISR          |

## Develop

```sh
cargo test                  # host workspace
cargo check                 # api + core + types (host)
trunk serve --config admin/Trunk.toml   # CRM SPA (wasm)
pnpm --dir web dev          # public site
```

`admin/` is excluded from `default-members`: it targets `wasm32` and is built
with `trunk`, so host `cargo` commands stay fast and green.
