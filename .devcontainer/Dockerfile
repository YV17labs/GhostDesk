FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

ENV DEBIAN_FRONTEND=noninteractive \
    DISPLAY=:99 \
    SCREEN_WIDTH=1280 \
    SCREEN_HEIGHT=800 \
    SCREEN_DEPTH=24 \
    VNC_PASSWORD=changeme \
    PORT=3000 \
    GTK_MODULES=gail:atk-bridge \
    GNOME_ACCESSIBILITY=1

# Add Mozilla APT repo for real Firefox deb (Ubuntu ships a snap wrapper)
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && install -d -m 0755 /etc/apt/keyrings \
    && curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg \
       -o /etc/apt/keyrings/packages.mozilla.org.asc \
    && echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
       > /etc/apt/sources.list.d/mozilla.list \
    && apt-get update \
    && apt-get remove -y --purge firefox \
    && apt-get install -y --no-install-recommends firefox/mozilla \
    && rm -rf /var/lib/apt/lists/*

# Desktop stack + tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    xvfb \
    openbox \
    x11vnc \
    novnc \
    websockify \
    xdotool \
    maim \
    xclip \
    hsetroot \
    supervisor \
    dbus-x11 \
    at-spi2-core \
    gir1.2-atspi-2.0 \
    python3-gi \
    && rm -rf /var/lib/apt/lists/*

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Supervisor config
COPY supervisor.conf /etc/supervisor/conf.d/ghostdesk.conf

# Openbox minimal config
RUN mkdir -p /home/vscode/.config/openbox \
    && printf '<?xml version="1.0" encoding="UTF-8"?>\n<openbox_config xmlns="http://openbox.org/3.4/rc">\n  <applications>\n    <application class="*"><decor>no</decor></application>\n  </applications>\n</openbox_config>\n' > /home/vscode/.config/openbox/rc.xml \
    && chown -R vscode:vscode /home/vscode/.config

# Enable Firefox accessibility (AT-SPI) — required for list_elements / click_element tools
RUN mkdir -p /home/vscode/.mozilla/firefox/ghostdesk.default-release \
    && printf '[General]\nStartWithLastProfile=1\n\n[Profile0]\nName=default-release\nIsRelative=1\nPath=ghostdesk.default-release\nDefault=1\n' \
       > /home/vscode/.mozilla/firefox/profiles.ini \
    && printf 'user_pref("accessibility.force_disabled", 0);\nuser_pref("accessibility.AOM.enabled", true);\n' \
       > /home/vscode/.mozilla/firefox/ghostdesk.default-release/user.js \
    && chown -R vscode:vscode /home/vscode/.mozilla

EXPOSE 3000 5900 6080
