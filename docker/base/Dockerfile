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

# ---- Stage 2: vnc-builder — wayvnc + neatvnc + aml from pinned SHAs ----
# Ubuntu 24.04 ships wayvnc 0.7.2, which does not advertise RFB security
# type 2 (classic VNC Auth) to its clients. wayvnc added opt-in DES auth
# in commit 1497397fc4 (2026-04-06); neatvnc in 6109e61b04 (2026-03-08).
# Until these land in a tagged release, we build from pinned master SHAs.
# Replace with a tag once wayvnc ships one that contains 1497397fc4.
FROM ubuntu:${UBUNTU_VERSION} AS vnc-builder

ARG WAYVNC_SHA=854edbbde0673b22386086dc98d1c89bee86966e
ARG NEATVNC_SHA=8a161c26a45706b950023df8bd8b29048e37c210
ARG AML_SHA=ce4b82d5888a87110054b95414bd3068224d2b91

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates git patch \
        meson ninja-build pkg-config gcc \
        libdrm-dev libxkbcommon-dev libwayland-dev \
        libjansson-dev libpam0g-dev libgnutls28-dev \
        libavfilter-dev libavcodec-dev libavutil-dev libturbojpeg0-dev \
        libpixman-1-dev nettle-dev libgmp-dev \
        zlib1g-dev scdoc

WORKDIR /src
COPY docker/base/patches/ /patches/
RUN git clone https://github.com/any1/aml.git \
    && git -C aml checkout ${AML_SHA} \
    && git clone https://github.com/any1/neatvnc.git \
    && git -C neatvnc checkout ${NEATVNC_SHA} \
    && mkdir neatvnc/subprojects \
    && ln -s ../../aml neatvnc/subprojects/aml \
    && git clone https://github.com/any1/wayvnc.git \
    && git -C wayvnc checkout ${WAYVNC_SHA} \
    && mkdir wayvnc/subprojects \
    && ln -s ../../neatvnc wayvnc/subprojects/neatvnc \
    && ln -s ../../aml wayvnc/subprojects/aml \
    # Drop Apple Diffie-Hellman (RFB security type 30) from the list
    # neatvnc advertises so noVNC 1.6 falls through to classic VNC Auth
    # (type 2). See neatvnc/src/server.c `init_security_types`.
    && sed -i '/ADD_SECURITY_TYPE(RFB_SECURITY_TYPE_APPLE_DH)/d' \
        neatvnc/src/server.c \
    # Bridge wayvnc's on_auth to the async neatvnc API; wayvnc master is
    # still on the sync signature and would hang the RFB handshake after
    # auth. See the patch header for the full rationale.
    && patch -p1 -d wayvnc < /patches/wayvnc-async-auth.patch \
    && meson setup wayvnc/build wayvnc --prefix=/usr -Dbuildtype=release \
    && ninja -C wayvnc/build \
    && DESTDIR=/out ninja -C wayvnc/build install

# ---- Stage 3: novnc — upstream 1.6 (Ubuntu 24.04 ships 1.3) ----
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

# ---- Stage 4: runtime — Wayland stack + supervisord + hardened venv ----
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
        websockify \
        supervisor \
        python3 \
        fonts-dejavu \
        tzdata locales xdg-user-dirs \
        ca-certificates \
        libdrm2 libjansson4 libpam0g libgnutls30 \
        libavfilter9 libavcodec60 libavutil58 libturbojpeg \
        libpixman-1-0 libnettle8 libgmp10 zlib1g \
    && locale-gen en_US.UTF-8 C.UTF-8

# wayvnc (+ wayvncctl + libneatvnc + libaml shared libs) built from
# pinned master SHA — see vnc-builder stage. Master offers RFB type 2
# under allow_broken_crypto; the apt wayvnc 0.7.2 does not.
COPY --from=vnc-builder /out/usr/bin /usr/bin
COPY --from=vnc-builder /out/usr/lib /usr/lib
RUN ldconfig

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
      org.opencontainers.image.licenses="LicenseRef-FSL-1.1-ALv2" \
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
