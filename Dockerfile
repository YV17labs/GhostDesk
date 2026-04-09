FROM ghcr.io/yv17labs/ghostdesk:base

RUN apt-get update \
    \
    # Sudo (passwordless admin for agent)
    && apt-get install -y --no-install-recommends sudo \
    && echo "agent ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/agent \
    \
    # Terminal
    && apt-get install -y --no-install-recommends gnome-terminal \
    \
    # Firefox via Mozilla APT — Ubuntu ships a snap wrapper that doesn't work in containers
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && install -d -m 0755 /etc/apt/keyrings \
    && curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg \
        -o /etc/apt/keyrings/packages.mozilla.org.asc \
    && echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
        > /etc/apt/sources.list.d/mozilla.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends firefox/mozilla \
    \
    # Clean up APT cache
    && rm -rf /var/lib/apt/lists/*

# Copy GhostDesk application sources
COPY src /app/src
COPY pyproject.toml /app/pyproject.toml

# Install GhostDesk with all dependencies (including PyTorch + Ultralytics for UI detection)
RUN cd /app && pip install -q -e .

# Pre-download GPA-GUI-Detector model into the container
# This ensures the model is baked into the image and available offline
RUN mkdir -p /app/models && \
    curl -fsSL -o /app/models/model.pt \
    "https://huggingface.co/Salesforce/GPA-GUI-Detector/resolve/main/model.pt"
