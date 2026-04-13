# Auto-discover Wayland/sway env from the running sway service.
# Sourced by login shells via /etc/profile.d/. Lets developers who
# `docker exec -it` into the container use swaymsg, wl-copy, and the
# ghostdesk CLI without manual env setup. On a normal Linux host these
# vars come from pam_systemd at login; here they come from PID 1
# (docker/entrypoint.sh), which interactive shells don't inherit.
# SWAYSOCK embeds sway's PID (different every restart), so we glob it.
# Fallback defaults must match docker/entrypoint.sh, which exports the
# same values into supervisord's env for services.
: "${XDG_RUNTIME_DIR:=/run/user/1000}"
: "${WAYLAND_DISPLAY:=wayland-1}"
export XDG_RUNTIME_DIR WAYLAND_DISPLAY
if [ -z "${SWAYSOCK:-}" ] && [ -d "${XDG_RUNTIME_DIR}" ]; then
    for _s in "${XDG_RUNTIME_DIR}"/sway-ipc.*.sock; do
        [ -S "$_s" ] && export SWAYSOCK="$_s" && break
    done
    unset _s
fi
