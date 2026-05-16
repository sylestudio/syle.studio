#!/usr/bin/env bash
# Build all three artifacts into ./out for deployment to /opt/syle.
# Run on a host with: rust+nasm, node+pnpm, trunk.
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="$(pwd)/deploy/out"
rm -rf "$OUT"
mkdir -p "$OUT/bin"

echo ">> API (release)"
cargo build --release -p syle-api
cp target/release/syle-api "$OUT/bin/"

echo ">> CRM (trunk, wasm release)"
trunk build --release --config admin/Trunk.toml
cp -r admin/dist "$OUT/admin"

echo ">> Public site (Astro). API must be reachable for content."
: "${API_BASE:=http://127.0.0.1:8080}"
API_BASE="$API_BASE" pnpm --dir web install --frozen-lockfile
API_BASE="$API_BASE" pnpm --dir web build
cp -r web/dist "$OUT/web"

echo ">> Done. Sync $OUT to /opt/syle on the VPS."
