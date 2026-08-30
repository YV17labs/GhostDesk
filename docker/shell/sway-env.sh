# Auto-discover SWAYSOCK for interactive shells (docker exec -it).
# Sourced from /etc/profile.d/. XDG_RUNTIME_DIR, WAYLAND_DISPLAY, and
# the rest of the Wayland/DBus plumbing come from Dockerfile ENV so
# every process already has them — only SWAYSOCK needs runtime lookup
# because it embeds sway's PID and can't be a static ENV value.
#
# That PID is also what makes the socket verifiable: a name whose process is
# gone is a socket nothing answers on, and exporting one gives the shell a
# `swaymsg` that hangs up rather than an error it can read. sway-run clears
# those at every compositor start; this filter is what keeps a shell honest in
# the window before it does.
if [ -z "${SWAYSOCK:-}" ] && [ -d "${XDG_RUNTIME_DIR:-/run/user/1000}" ]; then
    for _s in "${XDG_RUNTIME_DIR}"/sway-ipc.*.sock; do
        [ -S "$_s" ] || continue
        _pid="${_s%.sock}"
        _pid="${_pid##*.}"
        case "${_pid}" in
            '' | *[!0-9]*) continue ;;
        esac
        if [ -d "/proc/${_pid}" ]; then
            export SWAYSOCK="$_s"
            break
        fi
    done
    unset _s _pid
fi
