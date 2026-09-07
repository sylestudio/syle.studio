# Deployment

The site runs on a single host (`vps2`) that already fronts several apps with
**nginx + certbot**. Deploys are driven by a **self-hosted GitHub Actions
runner** that lives on the box; pushing to `main` builds and ships all three
artifacts.

```
syle.studio        → Cloudflare (proxied) → nginx :443 → /opt/syle/web (static)
                                                        ├ /api/public/* → syle-api :8080
                                                        └ /media → syle-api :8080
admin.syle.studio  → Cloudflare (DNS-only) → nginx :443 → /opt/syle/admin (SPA)
                                                         ├ /api/*  → syle-api :8080
                                                         └ /media  → syle-api :8080
```

The API binds `127.0.0.1:8080` only; nginx is the sole public surface. Both
hostnames terminate TLS at nginx via Let's Encrypt certs (`certbot --nginx`).
The CRM is protected by the app's own Argon2id session auth.

## One-time host bootstrap (root)

Provisioned out of band (the runner user is unprivileged):

- `apt install nasm` (the AVIF encoder, `ravif`, needs it at build time).
- A `syle` system user + group; `github-runner` is added to the `syle` group.
- `/opt/syle/{bin,admin,web,nginx}` owned `github-runner:syle` (the runner
  deploys here without sudo); `/opt/syle/media` owned `syle:syle` (the API
  writes here). The two files under `/etc/nginx/sites-available/` are symlinks
  to `/opt/syle/nginx/*.conf`, so the full deploy updates versioned vhosts
  before its restricted nginx reload.
- A local Postgres role + database `syle`; the API runs `sqlx` migrations on
  startup, so an empty DB is provisioned on first boot.
- `/etc/syle/api.env` (`DATABASE_URL`, `MEDIA_DIR`, `RP_ID`, `RP_ORIGIN`),
  `chmod 600`, root-owned. `RP_ID`/`RP_ORIGIN` are **required** — set them
  before shipping the passkey-enabled binary (the API fails fast without them).
  Optionally `GITHUB_DISPATCH_TOKEN` + `GITHUB_REPO` enable "Publicar al sitio"
  (see **Content**); absent, that feature stays dormant.
- `syle-api.service` (this dir) → `/etc/systemd/system/`, `daemon-reload`,
  `enable`.
- A scoped `sudoers` rule letting `github-runner` run **only**
  `systemctl restart syle-api` and `systemctl reload nginx`.

## nginx + TLS

Install the two vhosts in `nginx/` to `/etc/nginx/sites-available/`, symlink
them into `sites-enabled/`, then issue certs (DNS must resolve to the host
first; for issuance keep the Cloudflare record DNS-only):

```sh
sudo certbot --nginx -d syle.studio -d www.syle.studio
sudo certbot --nginx -d admin.syle.studio
```

After issuance, flip the public records (`syle.studio`, `www`) to **proxied**
in Cloudflare for the CDN; leave `admin` DNS-only.

The zone contains unrelated proxied origins, so its global SSL mode is not the
control for this site. A Cloudflare **Configuration Rule** matching
`http.host in {"syle.studio" "www.syle.studio"}` sets SSL to **Full (strict)**.
Both names have valid Let's Encrypt certificates at the VPS. A **Single
Redirect** matches
`http.host in {"syle.studio" "www.syle.studio"} and (http.host eq "www.syle.studio" or not ssl)`
and sends a `301` to
`concat("https://syle.studio", http.request.uri.path)` with the query string
preserved. nginx mirrors the same canonical policy for direct-origin traffic.

`admin.syle.studio` stays DNS-only and returns `X-Robots-Tag: noindex` plus a
dedicated disallowing `robots.txt`. The public API is also explicitly noindex;
these exclusions do not change authentication or API behavior.
`apihackaton.syle.studio` has its own hostname-scoped Configuration Rule set to
**Full** (its origin certificate is for another hostname) and a Response Header
Transform Rule that sets `X-Robots-Tag: noindex, nofollow, noarchive`.

## Continuous deploy

`.github/workflows/deploy.yml` runs on the `vps2` runner on every push to
`main` (and `workflow_dispatch`): it builds the API (release), the CRM (trunk
→ wasm), and the public site (Astro, against the live API), drops the
artifacts into `/opt/syle`, restarts the API, and reloads nginx. Its final
smoke test verifies the exact site revision, the public API proxy, immutable
responsive-media caching, and a real 404 through the public vhost.

## Passkeys cutover

Passkey (WebAuthn) login is **additive** over the existing password auth:

1. Set `RP_ID`/`RP_ORIGIN` in `/etc/syle/api.env` **before** the deploy that
   ships the passkey binary, or the API won't boot.
2. The schema migration is non-blocking — `password_hash` becomes nullable and
   three tables are added; existing password login keeps working untouched.
3. After deploy, the operator signs in with their password, opens
   **Cuenta → Seguridad**, enrolls a passkey, and generates recovery codes
   (shown once — store them safely).
4. Break-glass if all factors are lost: re-seed the operator over SSH with
   `DATABASE_URL=… cargo run -p syle-api --example seed-admin -- <email> <pw>`.

## Content

The public site is static and reads the API **at build time** (`API_BASE`), so
content edits in the CRM don't show until the site is rebuilt. The full
`deploy.yml` rebuilds everything on push to `main`; for content-only changes the
CRM has a **"Publicar al sitio"** button that triggers a fast (~15s) rebuild of
only the static site via `.github/workflows/site.yml`.

How it works: the button calls `POST /api/admin/site/rebuild`; the API fires a
GitHub `repository_dispatch` (`rebuild-site`) that runs `site.yml` on the same
self-hosted runner (so it reuses the runner's existing perms — no new privilege).
`site.yml` shares the `deploy-vps2` concurrency group, so a content rebuild never
overlaps a full deploy's `rsync`. The CRM polls `GET /api/admin/site/status` for
progress.

One-time setup to enable it:

1. Create a **fine-grained GitHub PAT** scoped to this repo only, with
   permissions **Contents: Read and write** (to dispatch) and **Actions: Read**
   (to read run status). Nothing else.
2. Add to `/etc/syle/api.env` (root-owned, `chmod 600`): `GITHUB_DISPATCH_TOKEN=<pat>`
   and `GITHUB_REPO=eddndev-studio/syle.studio`. Restart `syle-api`.
3. `site.yml` must live on the **default branch** (`repository_dispatch` always
   runs the default-branch copy) — it does, once this lands on `main`.

The token is held server-side only and never reaches the browser; if it's
unset the API reports `Unconfigured` and the CRM hides the control.
