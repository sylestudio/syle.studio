# Deployment

The site runs on a single host (`vps2`) that already fronts several apps with
**nginx + certbot**. Deploys are driven by a **self-hosted GitHub Actions
runner** that lives on the box; pushing to `main` builds and ships all three
artifacts.

```
syle.studio        → Cloudflare (proxied) → nginx :443 → /opt/syle/web (static)
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
- `/opt/syle/{bin,admin,web}` owned `github-runner:syle` (the runner deploys
  here without sudo); `/opt/syle/media` owned `syle:syle` (the API writes here).
- A local Postgres role + database `syle`; the API runs `sqlx` migrations on
  startup, so an empty DB is provisioned on first boot.
- `/etc/syle/api.env` (`DATABASE_URL`, `MEDIA_DIR`), `chmod 600`, root-owned.
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

This Cloudflare zone's SSL/TLS mode is **Flexible**, so Cloudflare fetches the
origin over HTTP `:80`. The public vhost therefore serves the site on **both
`:80` and `:443` with no http→https redirect** (a `:80` redirect makes the CDN
loop). `admin` is DNS-only/direct and keeps certbot's `:80→:443` redirect.
Upgrading the zone to **Full (strict)** is recommended (CF↔origin would then be
encrypted) — but it is zone-wide, so first confirm the other proxied origins in
the zone present a valid cert on `:443`, or scope it per-host with a
Configuration Rule.

## Continuous deploy

`.github/workflows/deploy.yml` runs on the `vps2` runner on every push to
`main` (and `workflow_dispatch`): it builds the API (release), the CRM (trunk
→ wasm), and the public site (Astro, against the live API), drops the
artifacts into `/opt/syle`, restarts the API, and reloads nginx.

## Content

The public site is static and reads the API **at build time** (`API_BASE`),
so a publish in the CRM appears after the next deploy. A webhook-triggered
rebuild can be added later.
