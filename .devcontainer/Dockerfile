FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

ENV DEBIAN_FRONTEND=noninteractive \
    DISPLAY=:99 \
    SCREEN_WIDTH=1280 \
    SCREEN_HEIGHT=800 \
    SCREEN_DEPTH=24 \
    VNC_PASSWORD=changeme \
    PORT=3000 \
    RUN_USER=vscode \
    APP_DIR=/workspaces/ghostdesk

# Desktop environment (Firefox, Openbox, VNC, AT-SPI…)
COPY .docker/setup-desktop.sh /tmp/setup-desktop.sh
RUN bash /tmp/setup-desktop.sh && rm /tmp/setup-desktop.sh

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Supervisor config
COPY .docker/supervisor.conf /etc/supervisor/conf.d/ghostdesk.conf

# Wallpaper
COPY assets/wallpaper.png /usr/share/ghostdesk/wallpaper.png

# Openbox minimal config
RUN mkdir -p /home/vscode/.config/openbox \
    && printf '<?xml version="1.0" encoding="UTF-8"?>\n<openbox_config xmlns="http://openbox.org/3.4/rc">\n  <applications>\n    <application class="*"><decor>no</decor></application>\n  </applications>\n</openbox_config>\n' > /home/vscode/.config/openbox/rc.xml \
    && chown -R vscode:vscode /home/vscode/.config

EXPOSE 3000 5900 6080
