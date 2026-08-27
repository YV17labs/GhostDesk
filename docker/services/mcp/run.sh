#!/bin/sh
# MCP server wrapper — finds the binary, and nothing else.
#
# It waits for no socket, deliberately. The binary owns both waits, because in
# both cases it is the only party that knows what it needs:
#
#   - Sway's IPC socket: it globs $XDG_RUNTIME_DIR/sway-ipc.*.sock, keeps the
#     ones whose embedded PID is a live sway, and probes each until one
#     answers, re-discovering on any later failure. No SWAYSOCK is exported
#     here — a variable captured at start-up goes stale the moment Sway
#     restarts, which is the failure the in-process discovery exists to
#     survive.
#   - The Wayland display: the boot hook binds a virtual pointer and keyboard,
#     and retries the connect while the compositor is merely not up yet. That
#     retry *is* the real bind, so it cannot pass while the thing it needs is
#     absent — which a socket file test here could, and did.
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

echo "mcp-server: exec ${BIN}"
exec "${BIN}"
