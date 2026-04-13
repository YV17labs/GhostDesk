#!/bin/sh
# GhostDesk MCP server wrapper (supervisord [program:mcp-server]).
#
# supervisord's priority= ordering guarantees sway is *started* before
# mcp-server, but not that its IPC socket is ready. The sway socket name
# embeds sway's PID (`sway-ipc.$UID.$PID.sock`), so we must discover it via
# glob and export SWAYSOCK before exec'ing the server — otherwise the
# first `swaymsg -t get_tree` call from the screen/windows tool would race
# and fail.
#
# supervisord runs this as $GHOSTDESK_USER (user=%(ENV_GHOSTDESK_USER)s in
# supervisord.conf), so we do NOT wrap in runuser.
set -eu

BIN="${GHOSTDESK_DIR}/.venv/bin/ghostdesk"
if [ ! -x "${BIN}" ]; then
    echo "mcp-server: ${BIN} not found or not executable" >&2
    echo "mcp-server: init.d/05-uv-sync.sh should have created the venv" >&2
    exit 1
fi

RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/1000}"
TIMEOUT=30
elapsed=0

echo "mcp-server: waiting for sway IPC socket in ${RUNTIME_DIR}..."
while [ "${elapsed}" -lt "${TIMEOUT}" ]; do
    for sock in "${RUNTIME_DIR}"/sway-ipc.*.sock; do
        if [ -S "${sock}" ]; then
            export SWAYSOCK="${sock}"
            echo "mcp-server: sway ready, SWAYSOCK=${sock}"
            exec "${BIN}"
        fi
    done
    sleep 1
    elapsed=$((elapsed + 1))
done

echo "mcp-server: timeout after ${TIMEOUT}s waiting for sway IPC socket" >&2
exit 1
