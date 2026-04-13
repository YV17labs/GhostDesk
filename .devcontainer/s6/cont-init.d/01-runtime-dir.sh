#!/command/with-contenv sh
set -eu

# Create the canonical freedesktop.org XDG_RUNTIME_DIR before any
# service starts. On a normal systemd desktop pam_systemd creates
# /run/user/$UID automatically at login (mode 0700, tmpfs, owned by
# the user); inside this container there's no logind so we fabricate
# it ourselves. Everything else — D-Bus, Wayland, pipewire, sway IPC,
# wayvnc control — drops its socket here.
RUNTIME_DIR="/run/user/1000"
mkdir -p "${RUNTIME_DIR}"
chown 1000:1000 "${RUNTIME_DIR}"
chmod 0700 "${RUNTIME_DIR}"
