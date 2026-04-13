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

# Note: service env (HOME, XDG_RUNTIME_DIR, WAYLAND_DISPLAY, DBUS_*, …)
# is exported at boot by docker/entrypoint.sh (PID 1) and inherited by
# every supervisord child. This is how wayvnc gets HOME=/home/vscode
# instead of PID 1's HOME=/root — which is what caused wayvnc to crash
# reading /root/.config/wayvnc/config in the s6-era.
ENV DEBIAN_FRONTEND=noninteractive \
    TZ=America/Montreal \
    LANG=fr_CA.UTF-8 \
    GHOSTDESK_DIR=/workspaces/ghostdesk

# ---------- Wayland stack (mirrors Dockerfile.base runtime) + dev extras ----------
# Prod (Dockerfile.base) installs the minimum to run the headless Wayland
# desktop. Dev adds: compilers and *-dev for pywayland wheels, curl for
# fetching the Mozilla APT key, a terminal (foot), a couple of GUI probes,
# and extra fonts for test rendering. `supervisor` is the PID-1 process
# supervisor (shared config: docker/supervisord.conf).
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        sway mako-notifier \
        grim wl-clipboard \
        libwayland-client0 libxkbcommon0 \
        dbus dbus-user-session libglib2.0-bin \
        wayvnc novnc websockify \
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

# ---------- uv (Python tooling, kept for ghostdesk Python project) ----------
COPY --from=ghcr.io/astral-sh/uv:0.11 /uv /usr/local/bin/uv

# ---------- ghostdesk runtime (shared with prod — docker/) ----------
# supervisord.conf, entrypoint.sh, sway.config, mcp-run.sh and health are
# shared verbatim with Dockerfile.base. Behaviour differences:
#
#   - GHOSTDESK_USER and the Wayland/DBus/WLR env are declared in
#     .devcontainer/docker-compose.yml (environment:). Prod sets them
#     via Dockerfile.base ENV.
#   - Autostart: this Dockerfile sets CMD ["sleep","infinity"], which
#     sends entrypoint.sh down its `$# > 0` branch: init runs, then
#     sleep. The developer launches supervisord manually via the
#     "GhostDesk: Start stack" VS Code task (plain `supervisord -n`,
#     no sudo — pidfile and socket live in /tmp).
COPY docker/supervisord.conf    /etc/supervisord.conf
COPY docker/entrypoint.sh       /usr/local/bin/entrypoint
COPY docker/mcp-run.sh          /usr/local/bin/mcp-run
COPY docker/health              /usr/local/bin/health
COPY docker/sway.config         /etc/ghostdesk/sway.config
RUN chmod +x /usr/local/bin/entrypoint \
             /usr/local/bin/mcp-run \
             /usr/local/bin/health

# Auto-discover WAYLAND_DISPLAY / XDG_RUNTIME_DIR / SWAYSOCK in
# interactive shells — see docker/profile/sway-env.sh for details.
COPY docker/profile/sway-env.sh /etc/profile.d/sway-env.sh
RUN chmod 0644 /etc/profile.d/sway-env.sh

# Override the devcontainer base-image prompt — see docker/profile/bashrc-prompt.sh
# for why this is needed and what it does.
COPY docker/profile/bashrc-prompt.sh /etc/ghostdesk/bashrc-prompt.sh
RUN echo '. /etc/ghostdesk/bashrc-prompt.sh' >> /home/vscode/.bashrc

EXPOSE 3000 5900 6080

# entrypoint.sh runs init once and then exec's CMD. CMD is `sleep
# infinity` so the container stays alive without any service running —
# the developer launches supervisord manually via the VS Code task
# "GhostDesk: Start stack". See docker/entrypoint.sh `$# > 0` branch.
ENTRYPOINT ["/usr/local/bin/entrypoint"]
CMD ["sleep", "infinity"]
