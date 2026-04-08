#!/bin/bash
# Shared container init (called by entrypoint.sh and devcontainer postStartCommand).
set -euo pipefail

# --- Timezone -----------------------------------------------------------
TZ="${TZ:-UTC}"
if [ -f "/usr/share/zoneinfo/${TZ}" ]; then
    ln -snf "/usr/share/zoneinfo/${TZ}" /etc/localtime
    echo "${TZ}" > /etc/timezone
fi

# --- Locale -------------------------------------------------------------
LOCALE="${LOCALE:-en_US.utf8}"

# Normalise: accept both "fr_CA.utf8" and "fr_CA.UTF-8"
LOCALE_CANONICAL="${LOCALE/.utf8/.UTF-8}"
LOCALE_CANONICAL="${LOCALE_CANONICAL/.utf-8/.UTF-8}"

if ! locale -a 2>/dev/null | grep -qiF "${LOCALE}"; then
    sed -i "/${LOCALE_CANONICAL}/s/^# //" /etc/locale.gen
    locale-gen "${LOCALE_CANONICAL}"
    echo "init-container: generated ${LOCALE_CANONICAL}"
fi

update-locale LC_ALL="${LOCALE_CANONICAL}" LANG="${LOCALE_CANONICAL}"

# --- GNOME Keyring --------------------------------------------------------
# Pre-create an unlocked "Default keyring" with an empty password so that
# Electron / libsecret apps never prompt interactively.
KEYRING_DIR="/home/${RUN_USER:?}/.local/share/keyrings"
KEYRING_NAME="Default_keyring"
if [ ! -f "${KEYRING_DIR}/default" ]; then
    mkdir -p "${KEYRING_DIR}"
    printf '%s' "${KEYRING_NAME}" > "${KEYRING_DIR}/default"
    cat > "${KEYRING_DIR}/${KEYRING_NAME}.keyring" <<'KEYRING'
[keyring]
display-name=Default keyring
ctime=0
mtime=0
lock-on-idle=false
lock-after=false
KEYRING
    chmod 700 "${KEYRING_DIR}"
    chmod 600 "${KEYRING_DIR}/default" "${KEYRING_DIR}/${KEYRING_NAME}.keyring"
    chown -R "${RUN_USER}:${RUN_USER}" "${KEYRING_DIR}"
    echo "init-container: created default gnome-keyring"
fi
