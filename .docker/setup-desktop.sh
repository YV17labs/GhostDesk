#!/bin/bash
# Shared desktop environment setup for both devcontainer and production images.
set -euo pipefail

# --- Firefox from Mozilla APT repo (Ubuntu ships a snap wrapper) ---
apt-get update
apt-get install -y --no-install-recommends curl ca-certificates
install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg \
    -o /etc/apt/keyrings/packages.mozilla.org.asc
echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
    > /etc/apt/sources.list.d/mozilla.list
apt-get update
apt-get remove -y --purge firefox || true
apt-get install -y --no-install-recommends firefox/mozilla

# --- Desktop stack + dependencies ---
apt-get install -y --no-install-recommends \
    xvfb openbox tint2 x11vnc novnc websockify \
    xdotool maim xclip hsetroot x11-utils adwaita-icon-theme-full \
    locales \
    supervisor sudo \
    dbus-x11 \
    gnome-terminal \
    libgl1 libglib2.0-0 \
    tzdata

rm -rf /var/lib/apt/lists/*
