FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    DISPLAY=:99 \
    SCREEN_WIDTH=1280 \
    SCREEN_HEIGHT=1024 \
    SCREEN_DEPTH=24 \
    PORT=3000 \
    TZ=UTC \
    LOCALE=en_US.utf8 \
    RUN_USER=agent \
    APP_DIR=/app

# Desktop environment (Firefox, Openbox, VNC…)
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
COPY .docker/start-openbox.sh /usr/local/bin/start-openbox.sh
COPY .docker/init-container.sh /usr/local/bin/init-container.sh
COPY .docker/entrypoint.sh /usr/local/bin/entrypoint.sh
COPY assets/wallpaper_1280x1024.png /usr/share/ghostdesk/wallpaper.png
COPY .docker/tint2rc /usr/share/ghostdesk/tint2rc
RUN mkdir -p /home/agent/.config/openbox /home/agent/.config/tint2 \
    && printf '<?xml version="1.0" encoding="UTF-8"?>\n<openbox_config xmlns="http://openbox.org/3.4/rc">\n  <applications>\n    <application class="*"><decor>no</decor></application>\n  </applications>\n</openbox_config>\n' > /home/agent/.config/openbox/rc.xml \
    && cp /usr/share/ghostdesk/tint2rc /home/agent/.config/tint2/tint2rc \
    && chown -R agent:agent /home/agent/.config

EXPOSE 3000 5900 6080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:3000/mcp || exit 1

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
