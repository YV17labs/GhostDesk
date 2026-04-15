#!/bin/sh
# MCP server wrapper. sway's IPC socket name embeds sway's PID
# (`sway-ipc.$UID.$PID.sock`), so we discover it via glob and export
# SWAYSOCK before exec'ing the binary. Without this, the first
# `swaymsg -t get_tree` call would race sway startup.
# Runs as $GHOSTDESK_USER via supervisord's user=, so no runuser.
set -eu

BIN="${GHOSTDESK_DIR}/.venv/bin/ghostdesk"
if [ ! -x "${BIN}" ]; then
    echo "mcp-server: ${BIN} not found or not executable" >&2
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
