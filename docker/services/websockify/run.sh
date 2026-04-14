#!/bin/sh
# websockify wrapper. TLS is opt-in: with a cert+key mounted at
# /etc/ghostdesk/tls/server.{crt,key} we serve wss:// (--ssl-only),
# otherwise plain ws/http and TLS is expected upstream.
# For local dev with a browser-trusted cert, see SECURITY.md (mkcert).
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
