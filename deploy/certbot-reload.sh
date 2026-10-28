#!/bin/sh
set -eu
if [ "${RENEWED_LINEAGE:-}" = /etc/letsencrypt/live/syle.studio ]; then
    nginx -t
    systemctl reload nginx
fi
