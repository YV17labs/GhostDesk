# syntax=docker/dockerfile:1
# ghostdesk devcontainer — mirrors docker/base/Dockerfile runtime stack
# and adds the dev extras (compilers, *-dev, foot, GUI probes, extra
# fonts). Supervisord config and service wrappers are shared verbatim.

# ---- Stage 1: vnc-builder — wayvnc + neatvnc + aml from pinned SHAs ----
# Same pins as docker/base/Dockerfile; keep them in lockstep.
FROM ubuntu:24.04 AS vnc-builder

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
    # See docker/base/Dockerfile vnc-builder for the rationale.
    && sed -i '/ADD_SECURITY_TYPE(RFB_SECURITY_TYPE_APPLE_DH)/d' \
        neatvnc/src/server.c \
    && patch -p1 -d wayvnc < /patches/wayvnc-async-auth.patch \
    && meson setup wayvnc/build wayvnc --prefix=/usr -Dbuildtype=release \
    && ninja -C wayvnc/build \
    && DESTDIR=/out ninja -C wayvnc/build install

# ---- Stage 2: novnc — upstream 1.6 (Ubuntu 24.04 ships 1.3) ----
# Mirrors docker/base/Dockerfile § novnc. Isolated so the dev image
# stays free of curl+tarball clutter for the noVNC fetch itself.
FROM ubuntu:24.04 AS novnc
ARG NOVNC_VERSION=1.6.0
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && curl -fsSL "https://github.com/novnc/noVNC/archive/refs/tags/v${NOVNC_VERSION}.tar.gz" \
        | tar -xz -C /tmp \
    && mv "/tmp/noVNC-${NOVNC_VERSION}" /opt/novnc

COPY docker/services/novnc/index.html /opt/novnc/index.html
COPY assets/logo-mark.svg              /opt/novnc/app/images/ghostdesk-logo.svg

# ---- Stage 3: runtime — devcontainer base + prod runtime stack ----
FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

ARG TARGETARCH

ENV DEBIAN_FRONTEND=noninteractive \
    TZ=America/Montreal \
    LANG=fr_CA.UTF-8 \
    GHOSTDESK_DIR=/workspaces/ghostdesk

# Dev extras on top of the prod stack: compilers + *-dev for pywayland
# wheels, curl for the Mozilla APT key, foot, GUI probes, fonts.
# Runtime libs (libdrm2 etc.) feed the wayvnc binary copied from the
# vnc-builder stage — the apt wayvnc package is NOT installed here.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
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
        curl \
        python3-dev libwayland-dev pkg-config build-essential \
        foot \
        mousepad galculator \
        fonts-noto-color-emoji \
    && locale-gen en_US.UTF-8 fr_CA.UTF-8 C.UTF-8 \
    && rm -rf /var/lib/apt/lists/*

# wayvnc (+ wayvncctl + libneatvnc + libaml) built from pinned master
# SHA — see vnc-builder.
COPY --from=vnc-builder /out/usr/bin /usr/bin
COPY --from=vnc-builder /out/usr/lib /usr/lib
RUN ldconfig

COPY --from=novnc /opt/novnc /usr/share/novnc

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

COPY docker/services/foot/foot.ini /etc/xdg/foot/foot.ini
COPY docker/shell/bashrc-prompt.sh /etc/ghostdesk/bashrc-prompt.sh
RUN echo '. /etc/ghostdesk/bashrc-prompt.sh' >> /home/vscode/.bashrc

EXPOSE 3000 5900 6080

ENTRYPOINT ["/usr/local/bin/entrypoint"]
CMD ["sleep", "infinity"]
