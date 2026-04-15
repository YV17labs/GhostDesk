#!/bin/sh
# websockify wrapper. With a cert+key mounted, serve wss:// (--ssl-only).
# Auth lives one hop down, in wayvnc itself (RFB security type 2 under
# allow_broken_crypto). Without a cert: plain ws/http, no auth.
set -eu

TLS_CRT="${GHOSTDESK_TLS_CERT:-/etc/ghostdesk/tls/server.crt}"
TLS_KEY="${GHOSTDESK_TLS_KEY:-/etc/ghostdesk/tls/server.key}"

if [ -s "${TLS_CRT}" ] && [ -s "${TLS_KEY}" ]; then
    echo "websockify: TLS enabled (cert=${TLS_CRT})"
    exec websockify \
        --web=/usr/share/novnc \
        --cert="${TLS_CRT}" \
        --key="${TLS_KEY}" \
        --ssl-only \
        6080 127.0.0.1:5900
fi

echo "websockify: no TLS cert mounted — serving plain ws/http on :6080"
exec websockify \
    --web=/usr/share/novnc \
    6080 127.0.0.1:5900
