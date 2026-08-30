#!/bin/sh
# Compositor wrapper: clear the runtime dir of dead IPC sockets, then exec sway.
#
# sway names its IPC socket sway-ipc.<uid>.<pid>.sock and unlinks it on a clean
# exit — a SIGKILL, or a container whose supervisord is restarted, leaves it
# behind. In a devcontainer that is every session: the entrypoint runs once per
# container, the VS Code task starts supervisord many times. Consumers then
# glob a directory where most matches are dead and pick by luck — a `docker
# exec` shell exports a SWAYSOCK nothing listens on, and the one socket that
# does answer is not necessarily the one found first.
#
# Here rather than in the entrypoint on purpose: this runs on every compositor
# start, which is exactly when "dead" is knowable and when nothing has looked
# yet.
#
# The display socket needs none of this — libwayland flocks wayland-N.lock and
# reclaims a stale wayland-N itself.
set -eu

RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/1000}"

for sock in "${RUNTIME_DIR}"/sway-ipc.*.sock; do
    [ -S "${sock}" ] || continue

    pid="${sock%.sock}"
    pid="${pid##*.}"
    case "${pid}" in
        '' | *[!0-9]*) continue ;;
    esac

    # A live PID is left alone whatever it turns out to be: this removes a
    # socket only when its owner is gone.
    if [ -d "/proc/${pid}" ]; then
        continue
    fi

    rm -f "${sock}"
    echo "sway: removed stale IPC socket ${sock##*/} (pid ${pid} is gone)"
done

exec sway "$@"
