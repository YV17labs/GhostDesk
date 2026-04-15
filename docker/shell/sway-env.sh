# Auto-discover SWAYSOCK for interactive shells (docker exec -it).
# Sourced from /etc/profile.d/. XDG_RUNTIME_DIR, WAYLAND_DISPLAY, and
# the rest of the Wayland/DBus plumbing come from Dockerfile ENV so
# every process already has them — only SWAYSOCK needs runtime lookup
# because it embeds sway's PID and can't be a static ENV value.
if [ -z "${SWAYSOCK:-}" ] && [ -d "${XDG_RUNTIME_DIR:-/run/user/1000}" ]; then
    for _s in "${XDG_RUNTIME_DIR}"/sway-ipc.*.sock; do
        [ -S "$_s" ] && export SWAYSOCK="$_s" && break
    done
    unset _s
fi
