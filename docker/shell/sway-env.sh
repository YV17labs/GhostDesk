# Auto-discover Wayland env for interactive shells (docker exec -it).
# Sourced from /etc/profile.d/. On a real host pam_systemd sets these at
# login; here they come from PID 1 (docker/init/entrypoint.sh) and
# interactive shells do not inherit that env. Fallback defaults MUST
# match entrypoint.sh. SWAYSOCK embeds sway's PID, so we glob it.
: "${XDG_RUNTIME_DIR:=/run/user/1000}"
: "${WAYLAND_DISPLAY:=wayland-1}"
export XDG_RUNTIME_DIR WAYLAND_DISPLAY
if [ -z "${SWAYSOCK:-}" ] && [ -d "${XDG_RUNTIME_DIR}" ]; then
    for _s in "${XDG_RUNTIME_DIR}"/sway-ipc.*.sock; do
        [ -S "$_s" ] && export SWAYSOCK="$_s" && break
    done
    unset _s
fi
