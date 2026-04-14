# syntax=docker/dockerfile:1.7
# ghostdesk:latest — distribution image on top of :base.
# Adds Firefox + Wayland-native GUI apps (foot, mousepad, galculator),
# colour emoji fonts, and passwordless sudo for `agent`. See SECURITY.md
# for the sudo NOPASSWD threat model.

FROM ghcr.io/yv17labs/ghostdesk:base

ARG GHOSTDESK_VERSION=dev
ARG GHOSTDESK_GIT_SHA=unknown

USER root

# Firefox must come from packages.mozilla.org — Ubuntu's `firefox` package
# is a snap wrapper that does not work in containers.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        sudo \
        foot \
        mousepad \
        galculator \
        fonts-noto-color-emoji; \
    echo "agent ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/agent; \
    chmod 0440 /etc/sudoers.d/agent; \
    apt-get install -y --no-install-recommends curl; \
    install -d -m 0755 /etc/apt/keyrings; \
    curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg \
        -o /etc/apt/keyrings/packages.mozilla.org.asc; \
    echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
        > /etc/apt/sources.list.d/mozilla.list; \
    apt-get update; \
    apt-get install -y --no-install-recommends firefox/mozilla; \
    apt-get purge -y curl; \
    rm -rf /var/lib/apt/lists/*

LABEL org.opencontainers.image.title="ghostdesk" \
      org.opencontainers.image.description="Ghostdesk distribution image — MCP-controlled desktop with Firefox, foot, mousepad and galculator" \
      org.opencontainers.image.version="${GHOSTDESK_VERSION}" \
      org.opencontainers.image.revision="${GHOSTDESK_GIT_SHA}"
