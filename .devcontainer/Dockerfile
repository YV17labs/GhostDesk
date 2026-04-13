# syntax=docker/dockerfile:1

# ============================================================================
# Runtime image
#
# Input (mouse + keyboard) is handled entirely in-process via the
# Wayland virtual-pointer / virtual-keyboard protocols — see
# ``src/ghostdesk/input/_wayland.py``. No external binary, no
# ``/dev/uinput`` access, no kernel input devices required.
# ============================================================================
FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

ARG TARGETARCH
ARG S6_OVERLAY_VERSION=3.2.2.0

# Note: service env (HOME, XDG_RUNTIME_DIR, WAYLAND_DISPLAY, DBUS_*, WLR_*,
# …) is NOT set here. It is written to /run/s6/container_environment at
# boot by .devcontainer/s6/cont-init.d/00-service-env.sh and loaded into
# every service by `with-contenv`. This is deliberate: the canonical
# s6-overlay way, and the only way to set a per-service HOME that differs
# from PID 1's HOME=/root — which is what caused wayvnc to crash reading
# /root/.config/wayvnc/config as user vscode.
ENV DEBIAN_FRONTEND=noninteractive \
    TZ=America/Montreal \
    LOCALE=fr_CA.utf8 \
    RUN_USER=vscode \
    APP_DIR=/workspaces/ghostdesk \
    S6_CMD_WAIT_FOR_SERVICES_MAXTIME=30000

# ---------- Wayland stack + freedesktop services + test apps ----------
# Input is injected via the Wayland virtual-pointer/keyboard protocols
# from Python (pywayland → libwayland-client → compositor). Screen
# capture uses grim. Clipboard uses wl-clipboard.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        xz-utils curl ca-certificates \
        pkg-config python3-dev \
        dbus dbus-user-session libglib2.0-bin \
        sway foot kitty \
        grim wl-clipboard libxkbcommon0 \
        libwayland-client0 libwayland-dev \
        mako-notifier \
        xdg-desktop-portal xdg-desktop-portal-wlr \
        pipewire wireplumber \
        at-spi2-core \
        wayvnc novnc websockify \
        gnome-text-editor gnome-calculator \
        fonts-dejavu fonts-noto-color-emoji \
        xdg-user-dirs tzdata locales \
    && rm -rf /var/lib/apt/lists/*

# ---------- ghostdesk wallpaper (shown when no app is open) ----------
RUN install -d -m 0755 /usr/share/ghostdesk
COPY assets/wallpaper_1280x1024.png /usr/share/ghostdesk/wallpaper.png

# ---------- Firefox via Mozilla APT (Ubuntu snap wrapper is unusable in containers) ----------
RUN install -d -m 0755 /etc/apt/keyrings \
    && curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg \
        -o /etc/apt/keyrings/packages.mozilla.org.asc \
    && echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
        > /etc/apt/sources.list.d/mozilla.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends firefox/mozilla \
    && rm -rf /var/lib/apt/lists/*

# ---------- s6-overlay (PID 1, service supervision) ----------
RUN set -eux; \
    case "${TARGETARCH:-amd64}" in \
        amd64)  S6_ARCH=x86_64 ;; \
        arm64)  S6_ARCH=aarch64 ;; \
        *) echo "unsupported arch: ${TARGETARCH}"; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-noarch.tar.xz" -o /tmp/s6-noarch.tar.xz; \
    curl -fsSL "https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-${S6_ARCH}.tar.xz" -o /tmp/s6-arch.tar.xz; \
    tar -C / -Jxpf /tmp/s6-noarch.tar.xz; \
    tar -C / -Jxpf /tmp/s6-arch.tar.xz; \
    rm -f /tmp/s6-noarch.tar.xz /tmp/s6-arch.tar.xz

# ---------- uv (Python tooling, kept for ghostdesk Python project) ----------
COPY --from=ghcr.io/astral-sh/uv:0.11 /uv /usr/local/bin/uv

# ---------- s6 service definitions ----------
COPY .devcontainer/s6/s6-rc.d/   /etc/s6-overlay/s6-rc.d/
COPY .devcontainer/s6/cont-init.d/ /etc/cont-init.d/
RUN chmod +x /etc/cont-init.d/*.sh \
    && find /etc/s6-overlay/s6-rc.d -type f -name run -exec chmod +x {} \;

# Auto-discover WAYLAND_DISPLAY / XDG_RUNTIME_DIR / SWAYSOCK in
# interactive shells — see .devcontainer/sway-env.sh for details.
COPY .devcontainer/sway-env.sh /etc/profile.d/sway-env.sh
RUN chmod 0644 /etc/profile.d/sway-env.sh

# Override the devcontainer base-image prompt — see .devcontainer/bashrc-prompt.sh
# for why this is needed and what it does.
COPY .devcontainer/bashrc-prompt.sh /etc/ghostdesk/bashrc-prompt.sh
RUN echo '. /etc/ghostdesk/bashrc-prompt.sh' >> /home/vscode/.bashrc

EXPOSE 5900 6080 3000
