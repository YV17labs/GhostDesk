#!/bin/sh
# Block until the compositor's display socket exists, then exec "$@".
#
# supervisord's priority= orders the *spawns*, not the readiness: every
# service starts inside the same second, and both mako and wayvnc exit(1) when
# the display is not there yet. supervisord's answer to an exit is to respawn,
# so without this gate a cold boot spends restart budget on a start-up
# ordering detail and logs a crash that means nothing.
#
# What this is NOT is a readiness proof. A socket file says the compositor
# bound its name, not that it will answer — that proof only exists inside a
# client's own connect. Which is why GhostDesk does not come through here: it
# retries its Wayland connect itself, for as long as this gate would have
# waited, and a retry that *is* the real bind cannot drift from what the bind
# needs. This gate is for the programs that die instead of waiting.
set -eu

if [ "$#" -eq 0 ]; then
    echo "wait-for-wayland: usage: wait-for-wayland <program> [args...]" >&2
    exit 2
fi

LABEL="${1##*/}"
RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/1000}"
SOCKET="${RUNTIME_DIR}/${WAYLAND_DISPLAY:-wayland-1}"
TIMEOUT=30
elapsed=0

if [ ! -S "${SOCKET}" ]; then
    echo "${LABEL}: waiting for ${SOCKET}..."
fi

while [ ! -S "${SOCKET}" ]; do
    if [ "${elapsed}" -ge "${TIMEOUT}" ]; then
        echo "${LABEL}: no Wayland display at ${SOCKET} after ${TIMEOUT}s" >&2
        exit 1
    fi
    sleep 1
    elapsed=$((elapsed + 1))
done

exec "$@"
