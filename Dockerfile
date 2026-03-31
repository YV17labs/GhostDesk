FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    DISPLAY=:99 \
    SCREEN_WIDTH=1280 \
    SCREEN_HEIGHT=800 \
    SCREEN_DEPTH=24 \
    PORT=3000 \
    RUN_USER=agent \
    APP_DIR=/app

# Desktop environment (Firefox, Openbox, VNC, AT-SPI…)
COPY .docker/setup-desktop.sh /tmp/setup-desktop.sh
RUN bash /tmp/setup-desktop.sh && rm /tmp/setup-desktop.sh

# Create non-root user with passwordless sudo
RUN useradd -m -s /bin/bash agent \
    && echo "agent ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/agent

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Install Python deps first (cache-friendly: only re-runs when deps change)
WORKDIR /app
COPY pyproject.toml uv.lock README.md ./
RUN chown -R agent:agent /app
USER agent
RUN uv sync --no-dev
USER root

# Application source (changes often, after deps layer)
COPY --chown=agent:agent src/ ./src/

# Config + assets (single layer)
COPY .docker/supervisor.conf /etc/supervisor/conf.d/ghostdesk.conf
COPY assets/wallpaper.png /usr/share/ghostdesk/wallpaper.png
RUN mkdir -p /home/agent/.config/openbox \
    && printf '<?xml version="1.0" encoding="UTF-8"?>\n<openbox_config xmlns="http://openbox.org/3.4/rc">\n  <applications>\n    <application class="*"><decor>no</decor></application>\n  </applications>\n</openbox_config>\n' > /home/agent/.config/openbox/rc.xml \
    && chown -R agent:agent /home/agent/.config

EXPOSE 3000 5900 6080

CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/ghostdesk.conf"]
