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
    xvfb openbox x11vnc novnc websockify \
    xdotool maim xclip hsetroot \
    supervisor sudo \
    dbus-x11 python3 python3-venv \
    python3-gi gir1.2-atspi-2.0 at-spi2-core dconf-cli \
    gnome-terminal

rm -rf /var/lib/apt/lists/*

# --- AT-SPI accessibility (system-level) ---
mkdir -p /etc/dconf/profile /etc/dconf/db/local.d
printf 'user-db:user\nsystem-db:local\n' > /etc/dconf/profile/user
printf '[org/gnome/desktop/interface]\ntoolkit-accessibility=true\n' > /etc/dconf/db/local.d/00-a11y
dconf update
