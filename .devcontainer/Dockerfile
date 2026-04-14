# syntax=docker/dockerfile:1
# ghostdesk devcontainer — mirrors docker/base/Dockerfile runtime stack
# and adds the dev extras (compilers, *-dev, foot, GUI probes, extra
# fonts). Supervisord config and service wrappers are shared verbatim.

FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

ARG TARGETARCH

ENV DEBIAN_FRONTEND=noninteractive \
    TZ=America/Montreal \
    LANG=fr_CA.UTF-8 \
    GHOSTDESK_DIR=/workspaces/ghostdesk

# Dev extras on top of the prod stack: compilers + *-dev for pywayland
# wheels, curl for the Mozilla APT key, foot, GUI probes, fonts.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        sway mako-notifier \
        grim wl-clipboard \
        libwayland-client0 libxkbcommon0 \
        dbus dbus-user-session libglib2.0-bin \
        wayvnc websockify \
        supervisor \
        python3 \
        fonts-dejavu \
        tzdata locales xdg-user-dirs \
        ca-certificates \
        curl \
        python3-dev libwayland-dev pkg-config build-essential \
        foot \
        mousepad galculator \
        fonts-noto-color-emoji \
    && locale-gen en_US.UTF-8 fr_CA.UTF-8 C.UTF-8 \
    && rm -rf /var/lib/apt/lists/*

# Ubuntu 24.04 ships noVNC 1.3; we pull 1.6 for bug fixes, clipboard
# handling, mobile input.
ARG NOVNC_VERSION=1.6.0
RUN curl -fsSL "https://github.com/novnc/noVNC/archive/refs/tags/v${NOVNC_VERSION}.tar.gz" \
        | tar -xz -C /tmp \
    && rm -rf /usr/share/novnc \
    && mv "/tmp/noVNC-${NOVNC_VERSION}" /usr/share/novnc

COPY docker/services/novnc/index.html /usr/share/novnc/index.html
COPY assets/logo-mark.svg              /usr/share/novnc/app/images/ghostdesk-logo.svg

RUN install -d -m 0755 /usr/share/ghostdesk
COPY assets/wallpaper.png /usr/share/ghostdesk/wallpaper.png

# Firefox must come from packages.mozilla.org — Ubuntu's `firefox` package
# is a snap wrapper that does not work in containers.
RUN install -d -m 0755 /etc/apt/keyrings \
    && curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg \
        -o /etc/apt/keyrings/packages.mozilla.org.asc \
    && echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
        > /etc/apt/sources.list.d/mozilla.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends firefox/mozilla \
    && rm -rf /var/lib/apt/lists/*

COPY --from=ghcr.io/astral-sh/uv:0.11 /uv /usr/local/bin/uv

# Shared runtime with prod. GHOSTDESK_USER and the Wayland/DBus/WLR env
# come from .devcontainer/docker-compose.yml in dev (prod bakes them in
# docker/base/Dockerfile ENV). CMD below sends entrypoint.sh down its
# `$# > 0` branch: init runs then sleeps; the developer starts the stack
# manually via the "GhostDesk: Start stack" VS Code task.
COPY docker/supervisor/supervisord.conf /etc/supervisord.conf
COPY docker/supervisor/conf.d/          /etc/supervisor/conf.d/
COPY docker/services/sway/config        /etc/ghostdesk/sway.config
COPY --chmod=0755 docker/init/entrypoint.sh         /usr/local/bin/entrypoint
COPY --chmod=0755 docker/init/healthcheck.sh        /usr/local/bin/health
COPY --chmod=0755 docker/services/mcp/run.sh        /usr/local/bin/mcp-run
COPY --chmod=0755 docker/services/mako/run.sh       /usr/local/bin/mako-run
COPY --chmod=0755 docker/services/websockify/run.sh /usr/local/bin/websockify-run
COPY --chmod=0644 docker/shell/sway-env.sh          /etc/profile.d/sway-env.sh

COPY docker/shell/bashrc-prompt.sh /etc/ghostdesk/bashrc-prompt.sh
RUN echo '. /etc/ghostdesk/bashrc-prompt.sh' >> /home/vscode/.bashrc

EXPOSE 3000 5900 6080

ENTRYPOINT ["/usr/local/bin/entrypoint"]
CMD ["sleep", "infinity"]
