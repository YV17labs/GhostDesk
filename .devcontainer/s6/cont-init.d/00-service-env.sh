#!/bin/sh
# NOTE: shebang is plain /bin/sh, NOT /command/with-contenv. This script
# *writes* the container_environment — it must not depend on it. Using
# plain sh lets us inherit PID 1's env, which is where Dockerfile `ENV`
# directives land (APP_DIR, RUN_USER, TZ, …). That's how we relay those
# values into container_environment for downstream services, with
# fallbacks so the script still works if they're unset.
set -eu

# Populate /run/s6/container_environment so that `with-contenv` injects
# a complete, explicit env into every service. This is the canonical
# s6-overlay way of managing service env — do NOT re-enable S6_KEEP_ENV
# as a shortcut: it causes PID 1's HOME=/root to leak into services
# running as `vscode`, which silently breaks anything that reads a
# config under $HOME (wayvnc being the first victim).
#
# Any new service-level env var belongs here, not in Dockerfile ENV.

ENV_DIR="/run/s6/container_environment"
mkdir -p "${ENV_DIR}"

set_env() {
    # $1 = name, $2 = value. s6-envdir reads these files verbatim,
    # so no trailing newline.
    printf '%s' "$2" > "${ENV_DIR}/$1"
}

# --- User identity ------------------------------------------------------
# Services run as ${RUN_USER} (uid 1000 in dev = vscode; prod = agent)
# via s6-setuidgid. s6-setuidgid does NOT rewrite HOME/USER/LOGNAME, so
# we have to. RUN_USER comes from Dockerfile ENV; default to vscode for
# safety if somehow unset.
RUN_USER_VAL="${RUN_USER:-vscode}"
set_env HOME      "/home/${RUN_USER_VAL}"
set_env USER      "${RUN_USER_VAL}"
set_env LOGNAME   "${RUN_USER_VAL}"
set_env RUN_USER  "${RUN_USER_VAL}"

# --- App paths ----------------------------------------------------------
# APP_DIR is where the ghostdesk source tree lives (bind-mounted in dev,
# baked into the image in prod). Downstream services read it to locate
# ${APP_DIR}/.venv/bin/ghostdesk.
set_env APP_DIR   "${APP_DIR:-/workspaces/ghostdesk}"

# --- PATH ---------------------------------------------------------------
# with-contenv starts from an empty env when S6_KEEP_ENV is unset, so
# PATH must be explicit or s6-setuidgid can't even find the binary.
set_env PATH      /command:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# --- XDG / Wayland / D-Bus ---------------------------------------------
# Use the canonical /run/user/$UID location from the freedesktop.org XDG
# Base Directory spec. On a normal Linux desktop pam_systemd would create
# this for us at login; in this container 01-runtime-dir.sh does it
# manually before any service starts.
set_env XDG_RUNTIME_DIR          /run/user/1000
set_env XDG_SESSION_TYPE         wayland
set_env WAYLAND_DISPLAY          wayland-1
set_env DBUS_SESSION_BUS_ADDRESS unix:path=/run/user/1000/bus

# --- wlroots (headless) -------------------------------------------------
set_env WLR_BACKENDS            headless
set_env WLR_HEADLESS_OUTPUTS    1
set_env WLR_LIBINPUT_NO_DEVICES 1
set_env WLR_RENDERER            pixman

# --- Locale / timezone --------------------------------------------------
set_env TZ     America/Montreal
set_env LOCALE fr_CA.utf8
