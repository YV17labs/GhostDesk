#!/bin/sh
# MCP server wrapper.
#
# The binary discovers Sway's IPC socket itself — it globs
# $XDG_RUNTIME_DIR/sway-ipc.*.sock, keeps the ones whose embedded PID is a
# live sway, and probes each until one answers, re-discovering on any later
# failure. So no SWAYSOCK is exported here: a variable captured at start-up
# goes stale the moment Sway restarts, which is exactly the failure the
# in-process discovery exists to survive.
#
# What this wrapper still does is wait for the compositor to exist at all.
# The server binds its virtual pointer and keyboard during boot and refuses
# to start without them, so launching before Sway is up would only burn
# supervisord restarts.
#
# Runs as $GHOSTDESK_USER via supervisord's user=, so no runuser.
set -eu

# Production installs the binary at a fixed path; a devcontainer runs
# whatever the developer last built.
for candidate in \
    /usr/local/bin/ghostdesk \
    "${GHOSTDESK_DIR:-/opt/ghostdesk}/target/release/ghostdesk" \
    "${GHOSTDESK_DIR:-/opt/ghostdesk}/target/debug/ghostdesk"
do
    if [ -x "${candidate}" ]; then
        BIN="${candidate}"
        break
    fi
done

if [ -z "${BIN:-}" ]; then
    echo "mcp-server: no ghostdesk binary found — run 'cargo build --release'" >&2
    exit 1
fi

RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/1000}"
TIMEOUT=30
elapsed=0

echo "mcp-server: waiting for sway IPC socket in ${RUNTIME_DIR}..."
while [ "${elapsed}" -lt "${TIMEOUT}" ]; do
    for sock in "${RUNTIME_DIR}"/sway-ipc.*.sock; do
        if [ -S "${sock}" ]; then
            echo "mcp-server: sway ready, exec ${BIN}"
            exec "${BIN}"
        fi
    done
    sleep 1
    elapsed=$((elapsed + 1))
done

echo "mcp-server: timeout after ${TIMEOUT}s waiting for sway IPC socket" >&2
exit 1
