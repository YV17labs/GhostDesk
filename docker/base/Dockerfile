# syntax=docker/dockerfile:1.7
# ghostdesk:base — headless Wayland desktop for a single LLM agent.
# See SECURITY.md for threat model, auth requirements, and rotation.

ARG UBUNTU_VERSION=26.04

# ---- Stage 1: builder — the Rust binary ----
# Built on the same Ubuntu release as the runtime stage. The output links
# only libc/libm/libgcc_s, so matching the glibc it will run against is the
# one compatibility constraint that matters.
FROM ubuntu:${UBUNTU_VERSION} AS builder

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# `cc` builds libwebp from vendored sources — there is no system libwebp to
# install, and no libwayland either: the compositor is driven from pure Rust.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        build-essential pkg-config \
        ca-certificates curl

# Install rustup with no toolchain, then let `rust-toolchain.toml` name it.
# The channel is pinned in exactly one file, so a bump is a one-line change
# and prod can never drift from what CI and the devcontainer resolve.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        -o /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --no-modify-path --default-toolchain none \
    && rm -f /tmp/rustup-init.sh

WORKDIR /src
COPY rust-toolchain.toml ./
RUN rustup toolchain install

COPY Cargo.toml Cargo.lock ./
COPY apps/ ./apps/
COPY crates/ ./crates/

# `--locked` makes the build fail rather than silently resolve a different
# dependency than the committed lockfile — a release must be reproducible
# from the tag. Both caches are mounts, so the binary has to be copied out
# of `target/` inside this same RUN or it disappears with the mount.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --release --locked --bin ghostdesk \
    && install -D -m 0755 target/release/ghostdesk /out/usr/local/bin/ghostdesk

# ---- Stage 2: vnc-builder — wayvnc + neatvnc + aml from pinned tags ----
# Ubuntu's wayvnc (0.7.2) does not advertise RFB security type 2 (classic
# VNC Auth), so these three are built from upstream release tags.
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
# Build aml, neatvnc and wayvnc as standalone projects (not as meson
# subprojects) so every .so / .pc file meson produces lands in /out at
# its real multiarch location — `lib/x86_64-linux-gnu` on amd64,
# `lib/aarch64-linux-gnu` on arm64, etc. — without any triplet
# hardcoded here. Each project is installed twice: first to the system
# `/usr` so the next meson setup resolves the previous one via
# pkg-config, then staged under `/out` for the runtime stage's single
# `COPY --from=vnc-builder /out/ /`.
RUN set -eux \
    && git clone --depth 1 --branch ${AML_REF} https://github.com/any1/aml.git \
    && git clone --depth 1 --branch ${NEATVNC_REF} https://github.com/any1/neatvnc.git \
    && git clone --depth 1 --branch ${WAYVNC_REF} https://github.com/any1/wayvnc.git \
    # Drop Apple Diffie-Hellman (RFB security type 30) from the list
    # neatvnc advertises. With wayvnc's `relax_encryption=true` the
    # NVNC_AUTH_REQUIRE_ENCRYPTION flag is unset, so neatvnc would
    # otherwise advertise Apple-DH and noVNC would pick it first and
    # fail (no Apple-DH client implementation). Removing it forces
    # noVNC to fall through to classic VNC Auth (type 2).
    && sed -i '/ADD_SECURITY_TYPE(RFB_SECURITY_TYPE_APPLE_DH)/d' \
        neatvnc/src/server.c \
    && for proj in aml neatvnc wayvnc; do \
        meson setup "$proj/build" "$proj" \
            --prefix=/usr -Dbuildtype=release \
        && ninja -C "$proj/build" \
        && ninja -C "$proj/build" install \
        && DESTDIR=/out ninja -C "$proj/build" install; \
    done

# ---- Stage 3: novnc — upstream tag (Ubuntu repos lag the upstream release) ----
# Isolated so the runtime image stays free of curl and build tooling.
FROM ubuntu:${UBUNTU_VERSION} AS novnc
ARG NOVNC_VERSION=1.7.0
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && curl -fsSL "https://github.com/novnc/noVNC/archive/refs/tags/v${NOVNC_VERSION}.tar.gz" \
        | tar -xz -C /tmp \
    && mv "/tmp/noVNC-${NOVNC_VERSION}" /opt/novnc

# Override upstream's index.html (a symlink to vnc.html) with a branded
# landing page that auto-forwards with the right query params.
COPY docker/services/novnc/index.html /opt/novnc/index.html
COPY assets/logo-mark.svg              /opt/novnc/app/images/ghostdesk-logo.svg

# ---- Stage 4: runtime — Wayland stack + supervisord + the server binary ----
FROM ubuntu:${UBUNTU_VERSION} AS runtime

ARG TARGETARCH
ARG GHOSTDESK_VERSION=dev
ARG GHOSTDESK_GIT_SHA=unknown

# Every framework variable carries this prefix: the server reads
# GHOSTDESK_HTTP__PORT where a stock NestRS app reads NESTRS_HTTP__PORT. It is
# a property of the deployment, so it has to be on the process before the
# binary starts — a `.env` file is read after the prefix has already chosen
# which cascade to read. Not overridable here the way a stock image's is:
# entrypoint.sh hands the server GHOSTDESK_* settings by name, and refuses to
# boot under a prefix that would leave them unread.
ENV NESTRS_ENV_PREFIX=GHOSTDESK

ENV DEBIAN_FRONTEND=noninteractive \
    TZ=America/New_York \
    LANG=en_US.UTF-8 \
    GHOSTDESK_USER=agent \
    GHOSTDESK_DIR=/opt/ghostdesk \
    PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    XDG_RUNTIME_DIR=/run/user/1000 \
    XDG_SESSION_TYPE=wayland \
    WAYLAND_DISPLAY=wayland-1 \
    DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
    WLR_BACKENDS=headless \
    WLR_HEADLESS_OUTPUTS=1 \
    WLR_LIBINPUT_NO_DEVICES=1 \
    WLR_RENDERER=pixman

# No compilers, no headers, and no language runtime for GhostDesk itself —
# the server is one statically-wired binary against libc. `python3` is here
# only because `websockify` is a Python program; nothing GhostDesk ships
# imports it. TZ/LANG are re-applied at boot by entrypoint.sh, so only the
# en_US/C fallbacks are pre-generated.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        sway mako-notifier \
        grim wl-clipboard \
        libxkbcommon0 \
        dbus dbus-user-session libglib2.0-bin \
        websockify \
        supervisor \
        python3 \
        fonts-dejavu \
        tzdata locales xdg-user-dirs gettext-base \
        ca-certificates \
        libdrm2 libjansson4 libpam0g libgnutls30 \
        libavfilter11 libavcodec62 libavutil60 libturbojpeg0 \
        libpixman-1-0 libnettle8 libgmp10 zlib1g \
    && locale-gen en_US.UTF-8 C.UTF-8

# wayvnc (+ wayvncctl + libneatvnc + libaml shared libs) built from
# pinned master SHAs — see vnc-builder stage. Master offers RFB type 2
# under allow_broken_crypto; the apt wayvnc 0.7.2 does not. A single
# `COPY /out/ /` lets meson's multiarch libdir land at the same path
# in the runtime image regardless of the target architecture.
COPY --from=vnc-builder /out/ /
RUN ldconfig

COPY --from=novnc /opt/novnc /usr/share/novnc

# The ubuntu base image ships a default `ubuntu` user at UID 1000 —
# remove it before claiming the UID for `agent`.
RUN userdel -r ubuntu 2>/dev/null || true \
    && groupadd -g 1000 agent \
    && useradd -m -u 1000 -g agent -s /bin/bash agent \
    && install -d -m 0755 -o agent -g agent \
         /home/agent/.config /home/agent/.config/sway \
    && install -d -m 0700 -o agent -g agent /home/agent/.config/wayvnc

COPY docker/supervisor/supervisord.conf /etc/supervisord.conf
COPY docker/supervisor/conf.d/          /etc/supervisor/conf.d/
COPY docker/services/sway/config        /etc/ghostdesk/sway.config
COPY --chmod=0755 docker/init/entrypoint.sh         /usr/local/bin/entrypoint
COPY --chmod=0755 docker/init/healthcheck.sh        /usr/local/bin/health
COPY --chmod=0755 docker/init/wait-for-wayland.sh   /usr/local/bin/wait-for-wayland
COPY --chmod=0755 docker/services/sway/run.sh       /usr/local/bin/sway-run
COPY --chmod=0755 docker/services/mcp/run.sh        /usr/local/bin/mcp-run
COPY --chmod=0755 docker/services/websockify/run.sh /usr/local/bin/websockify-run
COPY --chmod=0644 docker/shell/sway-env.sh          /etc/profile.d/sway-env.sh

RUN install -d -m 0755 /usr/share/ghostdesk
COPY assets/wallpaper.svg /usr/share/ghostdesk/wallpaper.svg

# Ubuntu 26.04 swaybg loads images through libglycin, which sandboxes
# its loaders in bubblewrap. Under Docker's default AppArmor profile
# bwrap fails at `pivot_root` with "Operation not permitted" — a string
# glycin's fallback heuristic doesn't recognize, so it keeps trying the
# sandbox and the wallpaper never renders (only the #1a1b2e fallback
# colour from the sway config shows). Shadow bwrap with a stub that
# emits the message glycin DOES recognize as "namespaces unavailable",
# forcing the no-sandbox path. The cost is one layer of defense-in-depth
# on a static PNG we control — acceptable since the agent never feeds
# untrusted images to glycin.
RUN printf '#!/bin/sh\necho "bwrap: No permissions to create a new namespace" >&2\nexit 1\n' \
        > /usr/local/bin/bwrap \
    && chmod 0755 /usr/local/bin/bwrap

RUN install -d -o agent -g agent /opt/ghostdesk

# The server: one root-owned, world-executable, non-writable binary.
#
# The Python build needed a whole hardening pass here — strip sources,
# byte-compile, then chmod a tree of thousands of files — because an
# interpreter loads whatever code is on disk at import time. A compiled
# binary has no such surface: `agent` can execute it and cannot modify it,
# and there is nothing else to protect. See SECURITY.md.
COPY --from=builder --chown=root:root --chmod=0555 \
     /out/usr/local/bin/ghostdesk /usr/local/bin/ghostdesk

LABEL org.opencontainers.image.title="ghostdesk" \
      org.opencontainers.image.description="MCP server controlling a headless Wayland desktop for LLM agents" \
      org.opencontainers.image.source="https://github.com/yv17labs/ghostdesk" \
      org.opencontainers.image.licenses="LicenseRef-FSL-1.1-ALv2" \
      org.opencontainers.image.version="${GHOSTDESK_VERSION}" \
      org.opencontainers.image.revision="${GHOSTDESK_GIT_SHA}" \
      org.opencontainers.image.vendor="YV17labs" \
      org.opencontainers.image.authors="Yoann Vanitou <yvanitou@gmail.com>" \
      org.opencontainers.image.base.name="docker.io/library/ubuntu:26.04"

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD /usr/local/bin/health

STOPSIGNAL SIGTERM
EXPOSE 3000 5900 6080
ENTRYPOINT ["/usr/local/bin/entrypoint"]
