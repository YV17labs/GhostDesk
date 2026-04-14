#!/bin/sh
# mako wrapper. supervisord's priority= starts sway before mako but does
# NOT wait for the Wayland socket; without this wait mako crashes on its
# first boot with "failed to create display".
# Runs as $GHOSTDESK_USER via supervisord's user=, so no runuser.
set -eu

RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/1000}"
DISPLAY_NAME="${WAYLAND_DISPLAY:-wayland-1}"
SOCKET="${RUNTIME_DIR}/${DISPLAY_NAME}"
TIMEOUT=30
elapsed=0

echo "mako: waiting for ${SOCKET}..."
while [ "${elapsed}" -lt "${TIMEOUT}" ]; do
    if [ -S "${SOCKET}" ]; then
        echo "mako: wayland socket ready, exec mako"
        exec mako
    fi
    sleep 1
    elapsed=$((elapsed + 1))
done

echo "mako: timeout after ${TIMEOUT}s waiting for ${SOCKET}" >&2
exit 1
