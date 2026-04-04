FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

ENV DEBIAN_FRONTEND=noninteractive \
    DISPLAY=:99 \
    SCREEN_WIDTH=1280 \
    SCREEN_HEIGHT=800 \
    SCREEN_DEPTH=24 \
    VNC_PASSWORD=changeme \
    PORT=3000 \
    TZ=UTC \
    LOCALE=en_US.utf8 \
    RUN_USER=vscode \
    APP_DIR=/workspaces/ghostdesk

# Desktop environment (Firefox, Openbox, VNC…)
COPY .docker/setup-desktop.sh /tmp/setup-desktop.sh
RUN bash /tmp/setup-desktop.sh && rm /tmp/setup-desktop.sh

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Supervisor config
COPY .docker/supervisor.conf /etc/supervisor/conf.d/ghostdesk.conf
COPY .docker/start-openbox.sh /usr/local/bin/start-openbox.sh
COPY .docker/init-container.sh /usr/local/bin/init-container.sh
COPY .docker/entrypoint.sh /usr/local/bin/entrypoint.sh

# Wallpaper + tint2 taskbar
COPY assets/wallpaper_1280x1024.png /usr/share/ghostdesk/wallpaper.png
COPY .docker/tint2rc /usr/share/ghostdesk/tint2rc

# Openbox + tint2 config
RUN mkdir -p /home/vscode/.config/openbox /home/vscode/.config/tint2 \
    && printf '<?xml version="1.0" encoding="UTF-8"?>\n<openbox_config xmlns="http://openbox.org/3.4/rc">\n  <applications>\n    <application class="*"><decor>no</decor></application>\n  </applications>\n</openbox_config>\n' > /home/vscode/.config/openbox/rc.xml \
    && cp /usr/share/ghostdesk/tint2rc /home/vscode/.config/tint2/tint2rc \
    && chown -R vscode:vscode /home/vscode/.config

EXPOSE 3000 5900 6080
