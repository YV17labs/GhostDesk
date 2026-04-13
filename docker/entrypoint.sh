#!/bin/bash
# ghostdesk PID 1 — shared by prod (Dockerfile.base) and dev (.devcontainer/
# Dockerfile). One flat script: env, secrets, runtime dir, locales, sway &
# wayvnc config, uv sync, then a two-mode hand-off.
#
# Mode is decided by whether the container was given a CMD:
#
#   - No CMD  → prod: exec supervisord (autostart)
#   - CMD set → dev:  exec the CMD (`sleep infinity`). Developer launches
#               supervisord manually via the "Start stack" VS Code task.
#
# The target user also differs: prod runs as `agent`, dev runs as
# `vscode`. Both are UID 1000 in their respective images. The active
# name is read from $GHOSTDESK_USER (set by Dockerfile.base ENV in prod, by
# docker-compose.yml environment: in dev), and everything downstream —
# $HOME, chown targets, runuser, supervisord's per-program `user=` —
# derives from it.
set -eu

# ============================================================================
# 1. Identity: GHOSTDESK_USER → HOME
# ============================================================================
export GHOSTDESK_USER="${GHOSTDESK_USER:-vscode}"
export HOME="/home/${GHOSTDESK_USER}"
export USER="${GHOSTDESK_USER}"
export LOGNAME="${GHOSTDESK_USER}"

# ============================================================================
# 2. Static service env
# ============================================================================
# GHOSTDESK_DIR is baked by each Dockerfile (/opt/ghostdesk in prod,
# /workspaces/ghostdesk in the devcontainer via compose).
export GHOSTDESK_DIR="${GHOSTDESK_DIR:-/opt/ghostdesk}"
export PATH="${GHOSTDESK_DIR}/.venv/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

# /run/user/1000 is fabricated below (no pam_systemd inside the container).
# Every Wayland / DBus / wayvnc socket lives there.
export XDG_RUNTIME_DIR=/run/user/1000
export XDG_SESSION_TYPE=wayland
export WAYLAND_DISPLAY=wayland-1
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus

export WLR_BACKENDS=headless
export WLR_HEADLESS_OUTPUTS=1
export WLR_LIBINPUT_NO_DEVICES=1
export WLR_RENDERER=pixman

# ============================================================================
# 3. Resolve *_FILE secrets
# ============================================================================
# Docker / Kubernetes secret convention: GHOSTDESK_X_FILE points to a
# mounted file whose contents become GHOSTDESK_X. Fatal if neither form
# is set — the container refuses to boot without real values.
die() {
    echo "entrypoint: FATAL $*" >&2
    exit 1
}

if [ -n "${GHOSTDESK_AUTH_TOKEN_FILE:-}" ]; then
    [ -r "${GHOSTDESK_AUTH_TOKEN_FILE}" ] \
        || die "GHOSTDESK_AUTH_TOKEN_FILE=${GHOSTDESK_AUTH_TOKEN_FILE} not readable"
    GHOSTDESK_AUTH_TOKEN="$(cat "${GHOSTDESK_AUTH_TOKEN_FILE}")"
fi
[ -n "${GHOSTDESK_AUTH_TOKEN:-}" ] \
    || die "GHOSTDESK_AUTH_TOKEN is required (or provide GHOSTDESK_AUTH_TOKEN_FILE)"
export GHOSTDESK_AUTH_TOKEN

if [ -n "${GHOSTDESK_VNC_PASSWORD_FILE:-}" ]; then
    [ -r "${GHOSTDESK_VNC_PASSWORD_FILE}" ] \
        || die "GHOSTDESK_VNC_PASSWORD_FILE=${GHOSTDESK_VNC_PASSWORD_FILE} not readable"
    GHOSTDESK_VNC_PASSWORD="$(cat "${GHOSTDESK_VNC_PASSWORD_FILE}")"
fi
[ -n "${GHOSTDESK_VNC_PASSWORD:-}" ] \
    || die "GHOSTDESK_VNC_PASSWORD is required (or provide GHOSTDESK_VNC_PASSWORD_FILE)"
export GHOSTDESK_VNC_PASSWORD

# ============================================================================
# 4. XDG_RUNTIME_DIR
# ============================================================================
# On a normal systemd desktop pam_systemd creates /run/user/$UID automatically
# at login (0700, tmpfs, owned by the user). Inside this container there's no
# logind so we fabricate it ourselves.
mkdir -p "${XDG_RUNTIME_DIR}"
chown "${GHOSTDESK_USER}:${GHOSTDESK_USER}" "${XDG_RUNTIME_DIR}"
chmod 0700 "${XDG_RUNTIME_DIR}"

# ============================================================================
# 5. Locale & timezone
# ============================================================================
# Both are re-applied on every boot so `docker restart` with a different
# LANG or TZ takes effect without a rebuild. `locale-gen` and
# `dpkg-reconfigure tzdata` are idempotent and near-instant when the
# target is already configured, so the cost of running them each boot is
# negligible.
LANG_VAL="${LANG:-en_US.UTF-8}"
echo "entrypoint: locale-gen ${LANG_VAL}"
locale-gen "${LANG_VAL}" >/dev/null 2>&1 \
    || echo "entrypoint: WARN locale-gen failed for ${LANG_VAL}" >&2
update-locale LANG="${LANG_VAL}"

TZ_VAL="${TZ:-America/New_York}"
if [ -f "/usr/share/zoneinfo/${TZ_VAL}" ]; then
    echo "entrypoint: dpkg-reconfigure tzdata (${TZ_VAL})"
    echo "${TZ_VAL}" > /etc/timezone
    dpkg-reconfigure -f noninteractive tzdata >/dev/null 2>&1
else
    echo "entrypoint: WARN unknown TZ '${TZ_VAL}', keeping image default" >&2
fi

# ============================================================================
# 6. Sway config
# ============================================================================
# Static content lives at /etc/ghostdesk/sway.config, baked at build time
# (COPY docker/sway.config /etc/ghostdesk/sway.config). We drop it into
# ${HOME}/.config/sway/config so sway finds it on start.
SWAY_CFG_DIR="${HOME}/.config/sway"
install -d -m 0755 -o "${GHOSTDESK_USER}" -g "${GHOSTDESK_USER}" "${SWAY_CFG_DIR}"
install -m 0644 -o "${GHOSTDESK_USER}" -g "${GHOSTDESK_USER}" \
    /etc/ghostdesk/sway.config "${SWAY_CFG_DIR}/config"

# ============================================================================
# 7. Wayvnc config (uses GHOSTDESK_VNC_PASSWORD resolved above)
# ============================================================================
# Same posture in dev and prod: auth required, RSA-AES transport.
# wayvnc's password auth only works over an encrypted channel — we use
# RSA-AES (rsa_private_key_file) because it needs no CA and no cert
# rotation. A 2048-bit key is generated once at first boot under
# ${HOME}/.config/wayvnc/rsa_key.pem and reused on subsequent boots.
WAYVNC_CFG_DIR="${HOME}/.config/wayvnc"
WAYVNC_CFG_FILE="${WAYVNC_CFG_DIR}/config"
WAYVNC_RSA_KEY="${WAYVNC_CFG_DIR}/rsa_key.pem"
install -d -m 0700 -o "${GHOSTDESK_USER}" -g "${GHOSTDESK_USER}" "${WAYVNC_CFG_DIR}"

if [ ! -s "${WAYVNC_RSA_KEY}" ]; then
    runuser -u "${GHOSTDESK_USER}" -- \
        openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
        -out "${WAYVNC_RSA_KEY}" >/dev/null 2>&1
    chmod 0600 "${WAYVNC_RSA_KEY}"
fi
(
    umask 077
    cat > "${WAYVNC_CFG_FILE}" <<EOF
address=127.0.0.1
port=5900
enable_auth=true
username=${GHOSTDESK_USER}
password=${GHOSTDESK_VNC_PASSWORD}
rsa_private_key_file=${WAYVNC_RSA_KEY}
EOF
)
chown "${GHOSTDESK_USER}:${GHOSTDESK_USER}" "${WAYVNC_CFG_FILE}"
chmod 0600 "${WAYVNC_CFG_FILE}"

# ============================================================================
# 7b. TLS cert for websockify (wss:// + https:// on :6080)
# ============================================================================
# End-to-end encryption is a product responsibility: websockify must never
# serve plain ws:// — even in dev. We keep a fixed path so prod deployments
# can mount a real cert (Let's Encrypt, internal CA, etc.) at the same
# location and this block becomes a no-op.
#
# Dev fallback: generate a self-signed RSA-2048 cert valid 10 years, with
# SANs covering localhost and the loopback addresses. The browser will
# show a one-time warning that the dev accepts manually — the connection
# is still cryptographically sound (modern algo, valid SAN, 2048-bit key),
# which is what an audit actually checks.
TLS_DIR=/etc/ghostdesk/tls
TLS_CRT="${TLS_DIR}/server.crt"
TLS_KEY="${TLS_DIR}/server.key"
install -d -m 0755 "${TLS_DIR}"

if [ ! -s "${TLS_CRT}" ] || [ ! -s "${TLS_KEY}" ]; then
    echo "entrypoint: generating self-signed TLS cert at ${TLS_DIR}"
    openssl req -x509 -newkey rsa:2048 -sha256 -days 3650 -nodes \
        -keyout "${TLS_KEY}" -out "${TLS_CRT}" \
        -subj "/CN=localhost/O=GhostDesk/OU=dev" \
        -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:::1" \
        >/dev/null 2>&1
fi
chown "${GHOSTDESK_USER}:${GHOSTDESK_USER}" "${TLS_CRT}" "${TLS_KEY}"
chmod 0644 "${TLS_CRT}"
chmod 0600 "${TLS_KEY}"

# ============================================================================
# 8. uv sync (devcontainer only — no-op in prod where venv is baked)
# ============================================================================
if command -v uv >/dev/null 2>&1 && [ -f "${GHOSTDESK_DIR}/pyproject.toml" ]; then
    echo "entrypoint: uv sync ${GHOSTDESK_DIR}/.venv..."
    ( cd "${GHOSTDESK_DIR}" && runuser -u "${GHOSTDESK_USER}" -- uv sync --frozen )
fi

# ============================================================================
# 9. Hand off
# ============================================================================
# Two modes, decided by whether a CMD was passed:
#
#   - $# == 0 → prod: autostart supervisord as PID 1.
#   - $# > 0  → dev:  exec CMD (`sleep infinity`). The developer launches
#                     supervisord manually via the VS Code "Start stack"
#                     task when they want the stack up. Compose env
#                     already makes every required var available to any
#                     shell inside the container, so no extra plumbing.
if [ $# -eq 0 ]; then
    echo "entrypoint: exec supervisord"
    exec /usr/bin/supervisord -c /etc/supervisord.conf
fi

echo "entrypoint: init complete, handing off to CMD ($*)"
exec "$@"
