#!/bin/bash
# ghostdesk PID 1 — shared by prod (docker/base/Dockerfile) and dev
# (.devcontainer/Dockerfile). Sets up env, runtime dir, locales, sway &
# wayvnc config, then either execs supervisord (no CMD) or hands off to
# CMD (dev `sleep infinity`, stack started via the VS Code task).
#
# $GHOSTDESK_USER (agent in prod, vscode in dev) drives $HOME, chown
# targets, runuser, and supervisord's per-program `user=`.
set -eu

# ---- Identity ----
export GHOSTDESK_USER="${GHOSTDESK_USER:-vscode}"
export HOME="/home/${GHOSTDESK_USER}"
export USER="${GHOSTDESK_USER}"
export LOGNAME="${GHOSTDESK_USER}"

# ---- Static service env ----
export GHOSTDESK_DIR="${GHOSTDESK_DIR:-/opt/ghostdesk}"
export PATH="${GHOSTDESK_DIR}/.venv/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

# No pam_systemd in the container — /run/user/1000 is fabricated below.
export XDG_RUNTIME_DIR=/run/user/1000
export XDG_SESSION_TYPE=wayland
export WAYLAND_DISPLAY=wayland-1
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus

export WLR_BACKENDS=headless
export WLR_HEADLESS_OUTPUTS=1
export WLR_LIBINPUT_NO_DEVICES=1
export WLR_RENDERER=pixman

# ---- TLS detection + conditional secrets ----
# Auth ≡ TLS. Two postures keyed off a mounted cert+key at
# /etc/ghostdesk/tls/server.{crt,key} (or GHOSTDESK_TLS_CERT/KEY):
#   - Cert present → prod: TLS + auth. AUTH_TOKEN and VNC_PASSWORD
#     are mandatory.
#   - No cert      → dev:  plain + no auth. A static token over
#     cleartext would be theater, so we unset it if supplied.
die() {
    echo "entrypoint: FATAL $*" >&2
    exit 1
}

TLS_DIR="/etc/ghostdesk/tls"
TLS_CRT="${GHOSTDESK_TLS_CERT:-${TLS_DIR}/server.crt}"
TLS_KEY="${GHOSTDESK_TLS_KEY:-${TLS_DIR}/server.key}"

if [ -s "${TLS_CRT}" ] && [ -s "${TLS_KEY}" ]; then
    TLS_ENABLED=1
    export GHOSTDESK_TLS_CERT="${TLS_CRT}"
    export GHOSTDESK_TLS_KEY="${TLS_KEY}"
    echo "entrypoint: TLS enabled (cert=${TLS_CRT})"
else
    TLS_ENABLED=0
    echo "entrypoint: no TLS cert at ${TLS_CRT} — dev posture: plain transport, auth disabled"
fi

if [ "${TLS_ENABLED}" = "1" ]; then
    [ -n "${GHOSTDESK_AUTH_TOKEN:-}" ] \
        || die "GHOSTDESK_AUTH_TOKEN is required when TLS is enabled"
    [ -n "${GHOSTDESK_VNC_PASSWORD:-}" ] \
        || die "GHOSTDESK_VNC_PASSWORD is required when TLS is enabled"
    export GHOSTDESK_AUTH_TOKEN GHOSTDESK_VNC_PASSWORD
else
    if [ -n "${GHOSTDESK_AUTH_TOKEN:-}" ]; then
        echo "entrypoint: WARN GHOSTDESK_AUTH_TOKEN ignored — TLS is off" >&2
        unset GHOSTDESK_AUTH_TOKEN
    fi
    if [ -n "${GHOSTDESK_VNC_PASSWORD:-}" ]; then
        echo "entrypoint: WARN GHOSTDESK_VNC_PASSWORD ignored — wayvnc auth needs VeNCrypt" >&2
        unset GHOSTDESK_VNC_PASSWORD
    fi
fi

# wayvnc is pinned to 127.0.0.1 (see Wayvnc config below). Warn loudly
# rather than silently ignore operator overrides.
if [ -n "${GHOSTDESK_VNC_ADDRESS:-}" ] && [ "${GHOSTDESK_VNC_ADDRESS}" != "127.0.0.1" ]; then
    echo "entrypoint: WARN GHOSTDESK_VNC_ADDRESS=${GHOSTDESK_VNC_ADDRESS} ignored — wayvnc is pinned to 127.0.0.1" >&2
fi

# ---- XDG_RUNTIME_DIR (no logind in containers) ----
mkdir -p "${XDG_RUNTIME_DIR}"
chown "${GHOSTDESK_USER}:${GHOSTDESK_USER}" "${XDG_RUNTIME_DIR}"
chmod 0700 "${XDG_RUNTIME_DIR}"

# ---- Locale & timezone ----
# Re-applied every boot so `docker restart` with a different LANG/TZ
# takes effect without a rebuild. Both calls are idempotent.
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

# ---- Sway config ----
SWAY_CFG_DIR="${HOME}/.config/sway"
install -d -m 0755 -o "${GHOSTDESK_USER}" -g "${GHOSTDESK_USER}" "${SWAY_CFG_DIR}"
install -m 0644 -o "${GHOSTDESK_USER}" -g "${GHOSTDESK_USER}" \
    /etc/ghostdesk/sway.config "${SWAY_CFG_DIR}/config"

# ---- Wayvnc config ----
# wayvnc is pinned to 127.0.0.1 regardless of operator env. The trust
# boundary is the container netns; browser reaches wayvnc only via
# websockify on :6080. Exposing the native VNC port outside the
# container would add attack surface for zero gain.
#
# NOTE on VeNCrypt sub-types: with cert+key, wayvnc advertises RFB
# security type 19 (VeNCrypt) with X509 sub-types. noVNC only supports
# VeNCrypt 0.2 "Plain" sub-type (core/rfb.js) — it works because the
# numeric ID for "Plain" (256) maps across both spaces. If this ever
# breaks on a wayvnc upgrade, fall back to enable_auth=false and rely
# on the loopback + websockify wss:// envelope; the trust boundary
# still holds.
#
# NOTE on RSA-AES: wayvnc implements AES-EAX (RFB type 5); the browser
# Web Crypto API only exposes GCM, so noVNC implements the "RA2ne"
# variant (type 6). Types 5 and 6 don't interop and noVNC drops the
# connection silently. Do NOT re-add rsa_private_key_file.
WAYVNC_CFG_DIR="${HOME}/.config/wayvnc"
WAYVNC_CFG_FILE="${WAYVNC_CFG_DIR}/config"
install -d -m 0700 -o "${GHOSTDESK_USER}" -g "${GHOSTDESK_USER}" "${WAYVNC_CFG_DIR}"

(
    umask 077
    if [ "${TLS_ENABLED}" = "1" ]; then
        cat > "${WAYVNC_CFG_FILE}" <<EOF
address=127.0.0.1
port=5900
enable_auth=true
username=${GHOSTDESK_USER}
password=${GHOSTDESK_VNC_PASSWORD}
private_key_file=${TLS_KEY}
certificate_file=${TLS_CRT}
EOF
    else
        cat > "${WAYVNC_CFG_FILE}" <<EOF
address=127.0.0.1
port=5900
enable_auth=false
EOF
    fi
)
chown "${GHOSTDESK_USER}:${GHOSTDESK_USER}" "${WAYVNC_CFG_FILE}"
chmod 0600 "${WAYVNC_CFG_FILE}"

# Leftover from the deprecated RSA-AES mode (see NOTE above).
rm -f "${WAYVNC_CFG_DIR}/rsa_key.pem" "${WAYVNC_CFG_DIR}/rsa_key.pem.pub"

# ---- uv sync (devcontainer only; no-op in prod) ----
if command -v uv >/dev/null 2>&1 && [ -f "${GHOSTDESK_DIR}/pyproject.toml" ]; then
    echo "entrypoint: uv sync ${GHOSTDESK_DIR}/.venv..."
    ( cd "${GHOSTDESK_DIR}" && runuser -u "${GHOSTDESK_USER}" -- uv sync --frozen )
fi

# ---- Hand off ----
if [ $# -eq 0 ]; then
    echo "entrypoint: exec supervisord"
    exec /usr/bin/supervisord -c /etc/supervisord.conf
fi

echo "entrypoint: init complete, handing off to CMD ($*)"
exec "$@"
