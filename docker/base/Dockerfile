# syntax=docker/dockerfile:1.7
# ghostdesk:base — headless Wayland desktop for a single LLM agent.
# See SECURITY.md for threat model, auth requirements, and rotation.

ARG UBUNTU_VERSION=24.04
ARG UV_VERSION=0.11

FROM ghcr.io/astral-sh/uv:${UV_VERSION} AS uv

# ---- Stage 1: builder — Python venv with full build toolchain ----
FROM ubuntu:${UBUNTU_VERSION} AS builder

ENV DEBIAN_FRONTEND=noninteractive \
    PYTHONDONTWRITEBYTECODE=1

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        python3 python3-dev \
        libwayland-dev pkg-config \
        build-essential \
        ca-certificates

COPY --from=uv /uv /usr/local/bin/uv

WORKDIR /opt/ghostdesk
# hatchling reads README.md at wheel-build time but its content is
# irrelevant. Stubbing keeps the `uv sync` layer cache-stable across doc edits.
COPY pyproject.toml uv.lock ./
COPY src/ ./src/
RUN echo 'See https://github.com/yv17labs/ghostdesk' > README.md \
    && uv sync --frozen --no-dev --compile-bytecode --no-editable

# ---- Stage 2: novnc — upstream 1.6 (Ubuntu 24.04 ships 1.3) ----
# Isolated so the runtime image stays free of curl and build tooling.
FROM ubuntu:${UBUNTU_VERSION} AS novnc
ARG NOVNC_VERSION=1.6.0
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && curl -fsSL "https://github.com/novnc/noVNC/archive/refs/tags/v${NOVNC_VERSION}.tar.gz" \
        | tar -xz -C /tmp \
    && mv "/tmp/noVNC-${NOVNC_VERSION}" /opt/novnc

# Override upstream's index.html (a symlink to vnc.html) with a branded
# landing page that auto-forwards with the right query params.
COPY docker/services/novnc/index.html /opt/novnc/index.html
COPY assets/logo-mark.svg              /opt/novnc/app/images/ghostdesk-logo.svg

# ---- Stage 3: runtime — Wayland stack + supervisord + hardened venv ----
FROM ubuntu:${UBUNTU_VERSION} AS runtime

ARG TARGETARCH
ARG GHOSTDESK_VERSION=dev
ARG GHOSTDESK_GIT_SHA=unknown

ENV DEBIAN_FRONTEND=noninteractive \
    TZ=America/New_York \
    LANG=en_US.UTF-8 \
    GHOSTDESK_USER=agent \
    GHOSTDESK_DIR=/opt/ghostdesk \
    PATH=/opt/ghostdesk/.venv/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# Runtime-only python3 (no -dev, no compilers). TZ/LANG are re-applied at
# boot by entrypoint.sh, so only en_US/C fallbacks are pre-generated.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
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
    && locale-gen en_US.UTF-8 C.UTF-8

COPY --from=novnc /opt/novnc /usr/share/novnc

# ubuntu:24.04 ships a default `ubuntu` user at UID 1000 — remove it
# before claiming the UID for `agent`.
RUN userdel -r ubuntu 2>/dev/null || true \
    && groupadd -g 1000 agent \
    && useradd -m -u 1000 -g agent -s /bin/bash agent

COPY docker/supervisor/supervisord.conf /etc/supervisord.conf
COPY docker/supervisor/conf.d/          /etc/supervisor/conf.d/
COPY docker/services/sway/config        /etc/ghostdesk/sway.config
COPY --chmod=0755 docker/init/entrypoint.sh         /usr/local/bin/entrypoint
COPY --chmod=0755 docker/init/healthcheck.sh        /usr/local/bin/health
COPY --chmod=0755 docker/services/mcp/run.sh        /usr/local/bin/mcp-run
COPY --chmod=0755 docker/services/mako/run.sh       /usr/local/bin/mako-run
COPY --chmod=0755 docker/services/websockify/run.sh /usr/local/bin/websockify-run
COPY --chmod=0644 docker/shell/sway-env.sh          /etc/profile.d/sway-env.sh

RUN install -d -m 0755 /usr/share/ghostdesk
COPY assets/wallpaper.png /usr/share/ghostdesk/wallpaper.png

RUN install -d -o agent -g agent /opt/ghostdesk
COPY --from=builder /opt/ghostdesk/.venv /opt/ghostdesk/.venv

# Harden the venv: sourceless .pyc, root-owned, 0555/0444. `agent` can
# import and execute, nothing running as `agent` can modify the code.
# See SECURITY.md for rationale.
RUN python3 -m compileall -qb /opt/ghostdesk/.venv \
    && find /opt/ghostdesk/.venv -type f -name '*.py' -delete \
    && find /opt/ghostdesk/.venv -type d -name __pycache__ -exec rm -rf {} + \
    && chown -R root:root /opt/ghostdesk/.venv \
    && find /opt/ghostdesk/.venv -type d -exec chmod 0555 {} + \
    && find /opt/ghostdesk/.venv -type f -exec chmod 0444 {} + \
    && find /opt/ghostdesk/.venv/bin -type f -exec chmod 0555 {} +

LABEL org.opencontainers.image.title="ghostdesk" \
      org.opencontainers.image.description="MCP server controlling a headless Wayland desktop for LLM agents" \
      org.opencontainers.image.source="https://github.com/yv17labs/ghostdesk" \
      org.opencontainers.image.licenses="AGPL-3.0-only" \
      org.opencontainers.image.version="${GHOSTDESK_VERSION}" \
      org.opencontainers.image.revision="${GHOSTDESK_GIT_SHA}" \
      org.opencontainers.image.vendor="YV17" \
      org.opencontainers.image.authors="Yoann Vanitou <yvanitou@gmail.com>" \
      org.opencontainers.image.base.name="docker.io/library/ubuntu:24.04"

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD /usr/local/bin/health

STOPSIGNAL SIGTERM
EXPOSE 3000 5900 6080
ENTRYPOINT ["/usr/local/bin/entrypoint"]
