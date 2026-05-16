# Deployment

Architecture: the public site is static behind the Cloudflare CDN; the CRM is
reachable **only** through a Cloudflare Tunnel (no public DNS for admin).

```
syle.studio        → Cloudflare CDN → Caddy → /opt/syle/web (static) + /media
admin.syle.studio  → Cloudflare Tunnel → cloudflared → Caddy :8081
                       ├─ /api/*  → syle-api (127.0.0.1:8080)
                       └─ /*      → /opt/syle/admin (CRM SPA)
```

## 1. Build artifacts

```sh
./deploy/build.sh            # produces deploy/out/{bin,admin,web}
rsync -a deploy/out/ syle@vps:/opt/syle/
```

Prerequisites on the build host: Rust stable + **nasm**, Node + pnpm, `trunk`.
The Astro build needs the API reachable (`API_BASE`) so pages get content.

## 2. Database + API service

```sh
sudo install -d -o syle -g syle /opt/syle/media
sudo install -D deploy/api.env.example /etc/syle/api.env   # then edit secrets
sudo chmod 600 /etc/syle/api.env
sudo install -D deploy/syle-api.service /etc/systemd/system/syle-api.service
sudo systemctl daemon-reload && sudo systemctl enable --now syle-api
```

## 3. Caddy (origin)

Install `deploy/Caddyfile` to `/etc/caddy/Caddyfile`, then
`sudo systemctl reload caddy`.

## 4. Cloudflare Tunnel (interactive — run these yourself)

These need your Cloudflare account, so run them in the session with a leading
`!` or directly on the VPS:

```sh
cloudflared tunnel login
cloudflared tunnel create syle-admin
# put the tunnel id + credentials path into deploy/cloudflared-config.yml,
# install it to /etc/cloudflared/config.yml
cloudflared tunnel route dns syle-admin admin.syle.studio
sudo cloudflared service install
```

`cloudflared` is not yet installed on this machine
(`sudo dnf install cloudflared` or the official repo).

## 5. Cloudflare CDN cache rules (dashboard)

- `syle.studio/media/*` and `/assets/*`: Cache Everything, Edge TTL a year
  (files are content-hashed and immutable).
- HTML: respect origin / short TTL so publishes appear quickly.

## 6. Publish revalidation

When the CRM publishes content, rebuild the static site (re-run step 1's
Astro build + rsync). A webhook-triggered rebuild can be added later.
