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
# Wayland / DBus / wlroots / XDG plumbing is declared as Dockerfile ENV
# in both docker/base/Dockerfile and .devcontainer/Dockerfile, so every
# process in the container (PID 1, supervisord, VS Code task shells,
# `docker exec`) inherits it directly from Docker — no re-export here.
# Only GHOSTDESK_DIR stays because it's an operator-overridable knob.
export GHOSTDESK_DIR="${GHOSTDESK_DIR:-/opt/ghostdesk}"
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

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
        echo "entrypoint: WARN GHOSTDESK_VNC_PASSWORD ignored — wayvnc auth is only enabled under TLS" >&2
        unset GHOSTDESK_VNC_PASSWORD
    fi
fi

# ---- Operator env → framework env ----
# GHOSTDESK_* is the deployment's contract (docker-compose.yml, README,
# SECURITY.md) and does not change. The server reads NestRS-namespaced
# variables — NESTRS_HTTP__* for the transport, NESTRS_GHOSTDESK__* for
# GhostDesk's own settings — so the translation happens once, here.
#
# Each assignment defers to an already-set NESTRS_* value, so an operator
# who prefers to speak the framework's own names directly can, and wins.
gd_map() {
    # gd_map <target> <value>  — set <target> unless it is already non-empty.
    eval "current=\${$1:-}"
    if [ -z "${current}" ] && [ -n "$2" ]; then
        export "$1=$2"
    fi
}

# Inside the container the MCP server must listen on every interface so
# Docker's port-publishing layer can reach it. A standalone `ghostdesk`
# invocation stays on 127.0.0.1 per the MCP transports spec — the override
# here only affects containerized runs.
gd_map NESTRS_HTTP__HOST "${GHOSTDESK_HOST:-0.0.0.0}"
gd_map NESTRS_HTTP__PORT "${GHOSTDESK_PORT:-}"
gd_map NESTRS_HTTP__CORS_ORIGINS "${GHOSTDESK_ALLOWED_ORIGINS:-}"

if [ "${TLS_ENABLED}" = "1" ]; then
    gd_map NESTRS_HTTP__TLS_CERT_FILE "${TLS_CRT}"
    gd_map NESTRS_HTTP__TLS_KEY_FILE "${TLS_KEY}"
fi

gd_map NESTRS_GHOSTDESK__AUTH_TOKEN "${GHOSTDESK_AUTH_TOKEN:-}"
gd_map NESTRS_GHOSTDESK__IDLE_TIMEOUT_SECS "${GHOSTDESK_IDLE_TIMEOUT:-}"

# The MCP transport refuses a Host header it does not know, which is what
# stops a page on an attacker's origin from pointing its own hostname at
# this container and POSTing to /mcp. The default covers loopback only, so
# a deployment reached under a real hostname has to name itself.
gd_map NESTRS_MCP__ALLOWED_HOSTS "${GHOSTDESK_ALLOWED_HOSTS:-}"

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
# Re-applied every boot so `docker restart` picks up a new LANG/TZ
# without a rebuild.
#
# Ubuntu 26.04's rust-coreutils panics inside icu_collator when
# locale-gen runs with LC_COLLATE pointing at the locale being
# generated (chicken/egg). locale-gen then exits 0 with nothing
# built, update-locale rejects, set -e kills PID 1. LC_ALL=C around
# both calls avoids the panic; verifying via `locale -a` instead of
# locale-gen's exit code catches any other silent failure and falls
# back to en_US.UTF-8 (always pre-built in the base image).
LANG_VAL="${LANG:-en_US.UTF-8}"
# `locale -a` reports codesets in glibc's internal form (`fr_CA.utf8`).
LANG_GLIBC="${LANG_VAL/.UTF-8/.utf8}"
locale_present() {
    LC_ALL=C locale -a 2>/dev/null | grep -qxF "${LANG_GLIBC}"
}
if locale_present; then
    echo "entrypoint: locale ${LANG_VAL} already present"
else
    echo "entrypoint: locale-gen ${LANG_VAL}"
    LC_ALL=C locale-gen "${LANG_VAL}" >/dev/null 2>&1 || true
    if ! locale_present; then
        echo "entrypoint: WARN ${LANG_VAL} not generated, falling back to en_US.UTF-8" >&2
        LANG_VAL="en_US.UTF-8"
    fi
fi
LC_ALL=C update-locale LANG="${LANG_VAL}"
export LANG="${LANG_VAL}"

TZ_VAL="${TZ:-America/New_York}"
if [ -f "/usr/share/zoneinfo/${TZ_VAL}" ]; then
    echo "entrypoint: dpkg-reconfigure tzdata (${TZ_VAL})"
    echo "${TZ_VAL}" > /etc/timezone
    dpkg-reconfigure -f noninteractive tzdata >/dev/null 2>&1
else
    echo "entrypoint: WARN unknown TZ '${TZ_VAL}', keeping image default" >&2
fi

# ---- Sway config ----
# The virtual output resolution is the single source of truth for both the
# compositor (this file) and the coordinate layer the agent sees. The server
# reads it from GhostdeskConfig, which is why it is mapped into the
# framework's namespace right beside the substitution — one value in
# docker-compose.yml, two consumers, no drift.
SWAY_CFG_DIR="${HOME}/.config/sway"
export GHOSTDESK_SCREEN_WIDTH="${GHOSTDESK_SCREEN_WIDTH:-1280}"
export GHOSTDESK_SCREEN_HEIGHT="${GHOSTDESK_SCREEN_HEIGHT:-1024}"
gd_map NESTRS_GHOSTDESK__SCREEN_WIDTH "${GHOSTDESK_SCREEN_WIDTH}"
gd_map NESTRS_GHOSTDESK__SCREEN_HEIGHT "${GHOSTDESK_SCREEN_HEIGHT}"
envsubst '${GHOSTDESK_SCREEN_WIDTH} ${GHOSTDESK_SCREEN_HEIGHT}' \
    < /etc/ghostdesk/sway.config > "${SWAY_CFG_DIR}/config"
chown "${GHOSTDESK_USER}:${GHOSTDESK_USER}" "${SWAY_CFG_DIR}/config"
chmod 0644 "${SWAY_CFG_DIR}/config"

# ---- Wayvnc config ----
# Pinned to 127.0.0.1. Under TLS, enable_auth + allow_broken_crypto +
# relax_encryption + password (no username) makes wayvnc advertise
# RFB security type 2 (classic VNC Auth) which noVNC supports
# directly — the browser prompts for a single password inside the
# noVNC overlay. The DES challenge/response is weak on its own but
# travels inside the wss:// envelope, so the effective posture is
# "password inside a TLS tunnel". Requires wayvnc built from
# pinned master SHA — see docker/base/Dockerfile vnc-builder stage.
WAYVNC_CFG_DIR="${HOME}/.config/wayvnc"
WAYVNC_CFG_FILE="${WAYVNC_CFG_DIR}/config"

(
    umask 077
    if [ "${TLS_ENABLED}" = "1" ]; then
        # No certificate_file/private_key_file here on purpose: setting
        # them makes wayvnc advertise RFB security type 19 (VeNCrypt),
        # which noVNC then picks first and fails on (the only
        # sub-type wayvnc offers is X509Plain/262 while noVNC only
        # supports Plain/256). Leaving them out keeps VNC Auth (type 2)
        # as the first type noVNC can actually negotiate. TLS for the
        # wire is handled by websockify one hop up.
        cat > "${WAYVNC_CFG_FILE}" <<EOF
address=127.0.0.1
port=5900
enable_auth=true
allow_broken_crypto=true
relax_encryption=true
password=${GHOSTDESK_VNC_PASSWORD}
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

rm -f "${WAYVNC_CFG_DIR}/rsa_key.pem" "${WAYVNC_CFG_DIR}/rsa_key.pem.pub"

# ---- Hand off ----
# No dependency sync step: prod ships a compiled binary, and in a
# devcontainer the developer runs `cargo build` when they mean to.
if [ $# -eq 0 ]; then
    echo "entrypoint: exec supervisord"
    exec /usr/bin/supervisord -c /etc/supervisord.conf
fi

echo "entrypoint: init complete, handing off to CMD ($*)"
exec "$@"
