# syntax=docker/dockerfile:1
# ghostdesk devcontainer — mirrors docker/base/Dockerfile's runtime stack and
# adds the Rust toolchain plus the GUI apps a developer wants on the desktop.
# Supervisord config and service wrappers are shared verbatim with prod.

ARG UBUNTU_VERSION=26.04

# Keep in lockstep with rust-toolchain.toml. rustup honours that file and
# would download the pinned channel on first use anyway; naming it here means
# the image ships with it already installed.
ARG RUST_VERSION=1.97.1

# ---- Stage 1: vnc-builder — wayvnc + neatvnc + aml from pinned refs ----
# Same pins as docker/base/Dockerfile; keep them in lockstep.
FROM ubuntu:${UBUNTU_VERSION} AS vnc-builder

ARG WAYVNC_REF=v0.10.1
ARG NEATVNC_REF=v1.0.1
ARG AML_REF=v1.0.0

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates git \
        meson ninja-build pkg-config gcc dpkg-dev \
        libdrm-dev libxkbcommon-dev libwayland-dev \
        libjansson-dev libpam0g-dev libgnutls28-dev \
        libavfilter-dev libavcodec-dev libavutil-dev libturbojpeg0-dev \
        libpixman-1-dev nettle-dev libgmp-dev \
        zlib1g-dev scdoc

WORKDIR /src
# See docker/base/Dockerfile vnc-builder for the rationale.
RUN set -eux \
    && git clone --depth 1 --branch ${AML_REF} https://github.com/any1/aml.git \
    && git clone --depth 1 --branch ${NEATVNC_REF} https://github.com/any1/neatvnc.git \
    && git clone --depth 1 --branch ${WAYVNC_REF} https://github.com/any1/wayvnc.git \
    && sed -i '/ADD_SECURITY_TYPE(RFB_SECURITY_TYPE_APPLE_DH)/d' \
        neatvnc/src/server.c \
    && for proj in aml neatvnc wayvnc; do \
        meson setup "$proj/build" "$proj" \
            --prefix=/usr -Dbuildtype=release \
        && ninja -C "$proj/build" \
        && ninja -C "$proj/build" install \
        && DESTDIR=/out ninja -C "$proj/build" install; \
    done

# ---- Stage 2: novnc — upstream tag (Ubuntu repos lag the upstream release) ----
FROM ubuntu:${UBUNTU_VERSION} AS novnc
ARG NOVNC_VERSION=1.7.0
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && curl -fsSL "https://github.com/novnc/noVNC/archive/refs/tags/v${NOVNC_VERSION}.tar.gz" \
        | tar -xz -C /tmp \
    && mv "/tmp/noVNC-${NOVNC_VERSION}" /opt/novnc

COPY docker/services/novnc/index.html /opt/novnc/index.html
COPY assets/logo-mark.svg              /opt/novnc/app/images/ghostdesk-logo.svg

# ---- Stage 3: runtime — devcontainer base + prod runtime stack + Rust ----
FROM mcr.microsoft.com/devcontainers/base:ubuntu26.04

ARG TARGETARCH
ARG RUST_VERSION

ENV DEBIAN_FRONTEND=noninteractive \
    TZ=America/Montreal \
    LANG=fr_CA.UTF-8 \
    GHOSTDESK_DIR=/workspaces/ghostdesk \
    XDG_RUNTIME_DIR=/run/user/1000 \
    XDG_SESSION_TYPE=wayland \
    WAYLAND_DISPLAY=wayland-1 \
    DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
    WLR_BACKENDS=headless \
    WLR_HEADLESS_OUTPUTS=1 \
    WLR_LIBINPUT_NO_DEVICES=1 \
    WLR_RENDERER=pixman

# The prod runtime stack, plus what a Rust developer needs on top:
# build-essential and pkg-config for the `cc`-built crates (libwebp is
# vendored, not a system package), mold for fast incremental links, and the
# GUI apps the agent drives.
#
# Note what is *not* here: no libwayland-dev, and no Python at all.
# `wayland-client` speaks the protocol from pure Rust, so the compositor is
# driven over a socket with no C library in the link line.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update \
    && apt-get install -y --no-install-recommends \
        sway mako-notifier \
        grim wl-clipboard \
        libxkbcommon0 \
        dbus dbus-user-session libglib2.0-bin \
        websockify \
        supervisor \
        fonts-dejavu \
        tzdata locales xdg-user-dirs gettext-base \
        ca-certificates \
        libdrm2 libjansson4 libpam0g libgnutls30 \
        libavfilter11 libavcodec62 libavutil60 libturbojpeg0 \
        libpixman-1-0 libnettle8 libgmp10 zlib1g \
        curl \
        build-essential pkg-config mold \
        foot \
        mousepad galculator \
        fonts-noto-color-emoji \
    && locale-gen en_US.UTF-8 fr_CA.UTF-8 C.UTF-8 \
    && rm -rf /var/lib/apt/lists/*

# wayvnc (+ wayvncctl + libneatvnc + libaml) built from pinned refs — see
# vnc-builder. A single `COPY /out/ /` keeps meson's multiarch libdir intact
# whatever the target architecture.
COPY --from=vnc-builder /out/ /
RUN ldconfig

COPY --from=novnc /opt/novnc /usr/share/novnc

RUN install -d -m 0755 /usr/share/ghostdesk
COPY assets/wallpaper.svg /usr/share/ghostdesk/wallpaper.svg

# Same bwrap stub as docker/base/Dockerfile — Docker's default AppArmor
# profile blocks bwrap's pivot_root with a message glycin doesn't recognize,
# so swaybg never falls back to the no-sandbox loader path and the wallpaper
# fails to render. Emit the message glycin DOES recognize as "namespaces
# unavailable" to force the no-sandbox path.
RUN printf '#!/bin/sh\necho "bwrap: No permissions to create a new namespace" >&2\nexit 1\n' \
        > /usr/local/bin/bwrap \
    && chmod 0755 /usr/local/bin/bwrap

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

# ---- Rust toolchain ----
# RUSTUP_HOME sits outside $HOME so the toolchain survives a rebuild of the
# home volume; CARGO_HOME stays per-user for the registry cache.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/home/vscode/.cargo \
    PATH=/home/vscode/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN install -d -m 0755 "${RUSTUP_HOME}" \
    && install -d -m 0755 -o vscode -g vscode "${CARGO_HOME}" \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh \
    && chmod 0755 /tmp/rustup-init.sh \
    && su vscode -c "/tmp/rustup-init.sh -y --no-modify-path \
            --default-toolchain ${RUST_VERSION} \
            --profile minimal -c clippy -c rustfmt -c rust-src" \
    && rm -f /tmp/rustup-init.sh \
    && chmod -R a+rX "${RUSTUP_HOME}"

# mold cuts incremental link time to a fraction of GNU ld's. It lives in the
# environment rather than in a committed .cargo/config.toml so the repo still
# builds on a machine that does not have mold installed.
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Shared runtime with prod. GHOSTDESK_USER and the Wayland/DBus/WLR env come
# from .devcontainer/docker-compose.yml in dev (prod bakes them in
# docker/base/Dockerfile ENV). CMD below sends entrypoint.sh down its
# `$# > 0` branch: init runs then sleeps, and the developer starts the stack
# with the "GhostDesk: Start stack" VS Code task.
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
RUN echo '. /etc/ghostdesk/bashrc-prompt.sh' >> /home/vscode/.bashrc \
    && install -d -m 0755 -o vscode -g vscode \
         /home/vscode/.config /home/vscode/.config/sway \
    && install -d -m 0700 -o vscode -g vscode /home/vscode/.config/wayvnc

EXPOSE 3000 5900 6080

ENTRYPOINT ["/usr/local/bin/entrypoint"]
CMD ["sleep", "infinity"]
