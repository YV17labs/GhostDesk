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

export LC_ALL="${LOCALE}" LANG="${LOCALE}"
