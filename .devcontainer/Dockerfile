FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

ENV DEBIAN_FRONTEND=noninteractive \
    DISPLAY=:99 \
    SCREEN_WIDTH=1280 \
    SCREEN_HEIGHT=1024 \
    SCREEN_DEPTH=24 \
    VNC_PASSWORD=changeme \
    PORT=3000 \
    TZ=UTC \
    LOCALE=en_US.utf8 \
    RUN_USER=vscode \
    APP_DIR=/workspaces/ghostdesk

# Base desktop stack
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        xvfb openbox tint2 x11vnc novnc websockify \
        xdotool maim xclip hsetroot x11-utils adwaita-icon-theme-full \
        locales \
        supervisor \
        dbus-x11 gnome-keyring libsecret-1-0 \
        libgl1 libglib2.0-0 \
        tzdata \
    \
    # Terminal
    && apt-get install -y --no-install-recommends gnome-terminal \
    \
    # Firefox via Mozilla APT — Ubuntu ships a snap wrapper that doesn't work in containers
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && install -d -m 0755 /etc/apt/keyrings \
    && curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg \
        -o /etc/apt/keyrings/packages.mozilla.org.asc \
    && echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
        > /etc/apt/sources.list.d/mozilla.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends firefox/mozilla \
    \
    # Clean up APT cache
    && rm -rf /var/lib/apt/lists/*

COPY --from=ghcr.io/astral-sh/uv:0.11 /uv /usr/local/bin/uv

COPY .docker/supervisor.conf /etc/supervisor/conf.d/ghostdesk.conf
COPY .docker/shared/start-openbox.sh /usr/local/bin/start-openbox.sh
COPY .docker/shared/init-container.sh /usr/local/bin/init-container.sh
COPY .docker/shared/entrypoint.sh /usr/local/bin/entrypoint.sh
COPY assets/wallpaper_1280x1024.png /usr/share/ghostdesk/wallpaper.png
COPY .docker/shared/tint2rc /usr/share/ghostdesk/tint2rc

RUN mkdir -p /home/vscode/.config/openbox /home/vscode/.config/tint2 \
    && printf '<?xml version="1.0" encoding="UTF-8"?>\n<openbox_config xmlns="http://openbox.org/3.4/rc">\n  <applications>\n    <application class="*"><decor>no</decor></application>\n  </applications>\n</openbox_config>\n' > /home/vscode/.config/openbox/rc.xml \
    && cp /usr/share/ghostdesk/tint2rc /home/vscode/.config/tint2/tint2rc \
    && chown -R vscode:vscode /home/vscode/.config

EXPOSE 3000 5900 6080
