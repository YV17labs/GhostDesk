FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    DISPLAY=:99 \
    SCREEN_WIDTH=1280 \
    SCREEN_HEIGHT=800 \
    SCREEN_DEPTH=24 \
    PORT=3000

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

# System dependencies — single layer, cache cleaned
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
    sudo \
    python3 \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user with passwordless sudo
RUN useradd -m -s /bin/bash agent \
    && echo "agent ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/agent

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Application setup
WORKDIR /app
COPY pyproject.toml uv.lock README.md ./
COPY src/ ./src/

# Install Python deps with uv (as agent user)
RUN chown -R agent:agent /app
USER agent
RUN uv sync --no-dev
USER root

# Supervisor config
COPY .docker/supervisor.conf /etc/supervisor/conf.d/ghostdesk.conf

# Openbox minimal config (no screensaver, no power manager)
RUN mkdir -p /home/agent/.config/openbox \
    && printf '<?xml version="1.0" encoding="UTF-8"?>\n<openbox_config xmlns="http://openbox.org/3.4/rc">\n  <applications>\n    <application class="*"><decor>no</decor></application>\n  </applications>\n</openbox_config>\n' > /home/agent/.config/openbox/rc.xml \
    && chown -R agent:agent /home/agent/.config

EXPOSE 3000 5900 6080

CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/ghostdesk.conf"]
