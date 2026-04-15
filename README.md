<p align="center">
  <img src="assets/logo.svg" alt="GhostDesk" width="200">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/MCP-compatible-blueviolet?style=for-the-badge" alt="MCP Compatible">
  <img src="https://img.shields.io/badge/python-3.12+-blue?style=for-the-badge&logo=python&logoColor=white" alt="Python 3.12+">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=for-the-badge" alt="AGPL-3.0 License">
  <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Docker-orange?style=for-the-badge&logo=docker&logoColor=white" alt="Platform">
</p>

<p align="center">
  <strong>Give your AI agent eyes, hands, and a full Linux desktop.</strong><br>
  An MCP server that lets LLM agents see the screen, move the mouse, type on the keyboard, launch apps, and run shell commands — all inside a sandboxed virtual desktop.
</p>

<p align="center">
  <em>If a human can do it on a desktop, your agent can too.</em>
</p>

<p align="center">
  <img src="demos/gifs/ghostdesk-amazon-sheets-automation.gif" alt="GhostDesk demo — Automated Amazon scraping to Google Sheets with real-time visualization" width="960">
</p>

---

## Why GhostDesk?

Browser automation tools (Playwright, Puppeteer, Selenium…) were built for human test engineers driving a browser with selectors. They do one thing, and they do it well — inside the browser.

GhostDesk is built from the other end: for **AI agents**, driving **everything a desktop runs**. Browsers, native apps, IDEs, terminals, office suites, legacy software, internal tools. If it renders pixels on screen, your agent can see it and use it — in one conversation, across many applications, without a line of glue code.

You don't write selectors. You write a prompt:

> *"Open the CRM, export last month's leads as CSV, open LibreOffice Calc, build a pivot table, screenshot the chart, and email it to the team."*

The agent opens the browser, logs in, downloads the file, switches to LibreOffice, processes the data, captures the result, composes the email, sends it. One prompt, multiple apps, fully autonomous — no glue code, no per-site scraper, no brittle selector chain.

That is what *agents using a desktop* looks like.

### Runs on models you can actually host

Desktop control needs to be **fast** — an agent that takes twelve seconds to decide where to click is unusable. GhostDesk's perception design (screenshots with region cropping, compact 12-tool surface, configurable coordinate space) is built so that vision-language models from the Qwen3-VL family running on a single workstation GPU are a first-class target, not an afterthought. No API bill, no screenshots of your desktop leaving your network.

Frontier models (Claude, GPT-4, Gemini) work too and remain the smoothest path — but they are not the bar. The project is explicitly designed so the medium, self-hosted tier delivers reliable results on real workflows.

---

## How it works

GhostDesk runs a virtual Linux desktop inside Docker and exposes it as an MCP server. Your agent gets a sandboxed desktop with a taskbar, clock, and pre-installed applications — equivalent to what a human sees on their screen.

The agent perceives the screen by calling `screen_shot()`, which captures the full desktop at native resolution and returns it as WebP (or PNG). An optional `region=` argument can crop to a sub-rectangle when the agent explicitly wants to narrow its focus.

This works with **any application** — web apps, native apps, legacy software, Canvas, WebGL. If it renders pixels, the agent can use it.

---

## Quick start

### 1. Run the container

GhostDesk couples **TLS and auth**: mount a cert and you get `wss://` + bearer-token on MCP + a single-password prompt on noVNC; mount nothing and every gate is disarmed on purpose (see [Security](#security) → *Auth ≡ TLS*). Even on localhost, the right procedure is to run the encrypted path end-to-end — [`mkcert`](https://github.com/FiloSottile/mkcert) issues a browser-trusted cert for `localhost` in two commands:

```bash
# Issue a locally-trusted cert (first time only — installs a local CA in your trust store)
mkcert -install
mkdir -p tls
mkcert -cert-file tls/server.crt -key-file tls/server.key localhost 127.0.0.1 ::1

# Generate the MCP and VNC secrets
export GHOSTDESK_AUTH_TOKEN=$(openssl rand -hex 32)
export GHOSTDESK_VNC_PASSWORD=$(openssl rand -hex 16)

# Run the container — cert mounted, TLS + auth enabled everywhere
docker run -d --name ghostdesk-my-agent \
  --restart unless-stopped \
  --cap-add SYS_ADMIN \
  -p 3000:3000 \
  -p 6080:6080 \
  -v ghostdesk-my-agent-home:/home/agent \
  -v "$PWD/tls/server.crt:/etc/ghostdesk/tls/server.crt:ro" \
  -v "$PWD/tls/server.key:/etc/ghostdesk/tls/server.key:ro" \
  --shm-size 2g \
  -e GHOSTDESK_AUTH_TOKEN \
  -e GHOSTDESK_VNC_PASSWORD \
  -e TZ=America/New_York \
  -e LANG=en_US.UTF-8 \
  ghcr.io/yv17labs/ghostdesk:latest

echo "MCP token:    $GHOSTDESK_AUTH_TOKEN"
echo "VNC password: $GHOSTDESK_VNC_PASSWORD"
```

Replace `my-agent` with whatever name fits your use case — `sales-agent`, `research-agent`, `accounting-agent`…

> **In production, swap the `mkcert` leaf for a real cert** (Let's Encrypt, your internal PKI, cert-manager…) mounted at the same path, and inject both secrets from your secret manager. On Kubernetes, use `valueFrom.secretKeyRef`; with Docker / compose, use a `.env` file backed by Docker secrets, Vault, AWS Secrets Manager, etc. See [Security](#security) for the full contract.

> **No cert, no auth.** If you skip the cert mount, GhostDesk boots in the dev posture: plain HTTP, no bearer-token gate, no VNC password — `GHOSTDESK_AUTH_TOKEN` and `GHOSTDESK_VNC_PASSWORD` are ignored with a warning. That shape is intended for IDE port-forwards (VS Code, Codespaces) where the forward layer already wraps the traffic; it is not something to point at any network you don't fully trust.

> **`--cap-add SYS_ADMIN`** — Required by Electron apps (VS Code, Slack, etc.) and other applications that need Linux user namespaces to run their sandbox. Safe to remove if you don't need them.

> **noVNC front-door authentication is still the operator's job.** The in-container VNC password is defense in depth — production deployments should also sit behind a reverse proxy / identity-aware proxy that terminates TLS and gates access to port 6080. See [Security](#security).

The named volume persists the agent's home directory across restarts — browser passwords, bookmarks, cookies, downloads, and desktop preferences are all preserved. On the first run, Docker automatically seeds the volume with the default configuration from the image.

### 2. Connect your AI

GhostDesk works with any MCP-compatible client. Add it to your config:

**Claude Desktop / Claude Code** (Streamable HTTP)
```json
{
  "mcpServers": {
    "ghostdesk": {
      "type": "http",
      "url": "https://localhost:3000/mcp",
      "headers": {
        "Authorization": "Bearer <your GHOSTDESK_AUTH_TOKEN>"
      }
    }
  }
}
```

**ChatGPT, Gemini, or any LLM with MCP support** — same config, same bearer token header.

### 3. Watch your agent work

Open `https://localhost:6080/vnc.html` in your browser to see the virtual desktop in real time. Because the cert you mounted was issued by `mkcert`'s local CA (installed in your trust store by `mkcert -install`), the browser accepts it with no warning. In production, swap the `mkcert` leaf for a real cert mounted at the same path, or terminate TLS on a reverse proxy in front of the container.

| Service | URL |
|---------|-----|
| MCP server | `https://localhost:3000/mcp` (behind a reverse proxy in production — see [Security](#security)) |
| noVNC (browser) | `https://localhost:6080/vnc.html` (username `agent`, password: `$GHOSTDESK_VNC_PASSWORD`) |
| VNC (loopback only — bound to `127.0.0.1` inside the container) | — |

---

## Tools

12 tools at your agent's fingertips, grouped by concern (`verb_noun` naming):

### Screen
| Tool | Description |
|------|-------------|
| `screen_shot` | Capture the screen as a WebP image (pass `format="png"` for lossless). Pass `region=` to crop to a sub-rectangle at native resolution. Set `stabilize=False` to skip page stabilization checks (default: True, waits max 5 sec for page to stabilize) |

### Mouse
| Tool | Description |
|------|-------------|
| `mouse_click` | Click at coordinates |
| `mouse_double_click` | Double-click at coordinates |
| `mouse_drag` | Drag from one position to another |
| `mouse_scroll` | Scroll in any direction (up/down/left/right) |

### Keyboard
| Tool | Description |
|------|-------------|
| `key_type` | Type text with realistic per-character delays |
| `key_press` | Press keys or combos (`ctrl+c`, `alt+F4`, `Return`...) |

### Clipboard
| Tool | Description |
|------|-------------|
| `clipboard_get` | Read clipboard contents |
| `clipboard_set` | Write to clipboard |

### Apps
| Tool | Description |
|------|-------------|
| `app_list` | List the GUI applications installed on the desktop |
| `app_launch` | Start a GUI application by name |
| `app_status` | Check if an application is running and read its logs |

---

## From one agent to a workforce

Each GhostDesk instance is a container. Spin up one, ten, or a hundred — each agent gets its own isolated desktop, its own apps, its own role. Think of it as hiring a team of digital employees, each with their own workstation.

### Scale horizontally

```yaml
# docker-compose.yml — 3 specialized agents, one command
#
# Secrets (GHOSTDESK_AUTH_TOKEN, GHOSTDESK_VNC_PASSWORD) come from a .env
# file (git-ignored) or your secret manager. On Kubernetes, wire them from
# a Secret via `valueFrom.secretKeyRef`. See SECURITY.md for the full
# secret-handling contract.
#
# TLS cert + key are mounted from ./tls on every service — generate once
# with `mkcert` for local runs, swap for a real cert in production:
#
#   mkcert -install
#   mkdir -p tls
#   mkcert -cert-file tls/server.crt -key-file tls/server.key \
#     localhost 127.0.0.1 ::1
#
# Mounting the cert flips GhostDesk into its prod posture: wss:// on both
# ports, bearer-token on MCP, single-password prompt on noVNC. See the
# Security section of this README for the Auth ≡ TLS rationale.

x-ghostdesk-base: &ghostdesk-base
  image: ghcr.io/yv17labs/ghostdesk:latest
  restart: unless-stopped
  cap_add: [SYS_ADMIN]
  shm_size: 2g
  environment:
    - GHOSTDESK_AUTH_TOKEN
    - GHOSTDESK_VNC_PASSWORD
    - LANG=en_US.UTF-8
    - TZ=America/New_York

services:
  sales-agent:
    <<: *ghostdesk-base
    container_name: ghostdesk-sales-agent
    ports: ["3001:3000", "6081:6080"]
    volumes:
      - ghostdesk-sales-agent-home:/home/agent
      - ./tls/server.crt:/etc/ghostdesk/tls/server.crt:ro
      - ./tls/server.key:/etc/ghostdesk/tls/server.key:ro

  research-agent:
    <<: *ghostdesk-base
    container_name: ghostdesk-research-agent
    ports: ["3002:3000", "6082:6080"]
    volumes:
      - ghostdesk-research-agent-home:/home/agent
      - ./tls/server.crt:/etc/ghostdesk/tls/server.crt:ro
      - ./tls/server.key:/etc/ghostdesk/tls/server.key:ro

  accounting-agent:
    <<: *ghostdesk-base
    container_name: ghostdesk-accounting-agent
    ports: ["3003:3000", "6083:6080"]
    volumes:
      - ghostdesk-accounting-agent-home:/home/agent
      - ./tls/server.crt:/etc/ghostdesk/tls/server.crt:ro
      - ./tls/server.key:/etc/ghostdesk/tls/server.key:ro

volumes:
  ghostdesk-sales-agent-home:
  ghostdesk-research-agent-home:
  ghostdesk-accounting-agent-home:
```

```bash
docker compose up -d   # Your workforce is ready
```

Each agent runs in parallel, independently, on its own desktop. Connect each to a different LLM, give each a different system prompt, install different apps — full specialization.

### Secure by design

Every agent is sandboxed in its own container. No access to the host machine. No access to other agents. Network, filesystem, and process isolation come free from Docker.

This makes GhostDesk a natural fit for enterprises:

| Concern | How GhostDesk handles it |
|---------|--------------------------|
| **Data isolation** | Each agent lives in its own container — no shared filesystem, no shared memory |
| **Access control** | Restrict network access per agent with Docker networking. An agent with CRM access doesn't see finance tools |
| **Auditability** | Watch any agent live via VNC, record sessions, review screenshots |
| **Blast radius** | If an agent goes wrong, kill the container. Nothing else is affected |
| **Compliance** | No data touches your host. Containers can run in air-gapped environments |

### Specialize each agent

Give each agent a role, like you would a new hire:

- **Sales agent** — monitors the CRM, enriches leads, updates the pipeline
- **Research agent** — browses the web, compiles competitive intelligence, writes reports
- **Accounting agent** — processes invoices in legacy ERP software, reconciles spreadsheets
- **QA agent** — clicks through your app, files bug reports with screenshots
- **Support agent** — handles tickets, looks up customer info across multiple internal tools

Each agent gets its own system prompt defining its mission, its own installed applications, and its own network permissions. Manage AI agents like employees — each with their own desktop, their own tools, and their own clearance level.

### Supervise in real time

Every agent exposes a VNC/noVNC endpoint. Open a browser tab and watch your agent work — or open ten tabs and monitor your entire workforce. Intervene at any time: take over the mouse, correct course, or chat with the orchestrating LLM.

---

## Model requirements

Your inference stack must cover four capabilities — all four are mandatory:

1. **Text + vision** — the agent perceives the desktop through screenshots and needs a model that can interpret them.
2. **Tool use** — GhostDesk exposes 12 tools as function calls; the model must be able to invoke them.
3. **MCP client** — the host needs to speak Streamable HTTP MCP to reach the GhostDesk server.
4. **WebP image support** — GhostDesk returns screenshots as WebP by default to keep payloads small and inference fast. A stack that can only decode PNG or JPEG will not work out of the box.

### Coordinate space — pick the right `GHOSTDESK_MODEL_SPACE` for your model

Vision models disagree on how they emit click coordinates, and this is the single setting that matters most for click accuracy. GhostDesk supports both conventions through `GHOSTDESK_MODEL_SPACE`:

| Value | Use for | Why |
|-------|---------|-----|
| `0` | **Frontier models** — Claude (all tiers including Haiku), GPT-4o, Gemini. | These models emit coordinates in native screen pixels directly. No remapping needed — they are sharp enough to target small icons without any normalisation layer in between. |
| `1000` | **Qwen vision family** — Qwen3.5, Qwen3-VL, and other Qwen vision models. | These models were trained to emit coordinates in a normalised 0-1000 space regardless of the input image size. GhostDesk rescales their output to screen pixels at the MCP boundary. Using `0` with these models produces clicks that are wildly off. |

Match the setting to the model and clicks land exactly where the model intended. Mismatch it and your agent will miss targets by half a screen — this is the most common source of "it kind of works but keeps missing" reports.

### Running locally

For self-hosted inference we use and recommend our fork of llama.cpp, which adds WebP decoding on top of upstream: [YV17labs/llama.cpp](https://github.com/YV17labs/llama.cpp). The day WebP lands upstream we will archive the fork and point there directly.

**Recommended models.** Desktop control needs *speed* more than raw intelligence — fast keyboard and mouse turns compound over a hundred-step workflow. Low-activation MoE vision models shine here: they stay responsive on modest hardware while reading icons sharply. Two solid picks from the Qwen vision family, both run with `GHOSTDESK_MODEL_SPACE=1000`:

- **[Qwen3.5-35B-A3B](https://huggingface.co/Qwen/Qwen3.5-35B-A3B)** — 35B parameters, only 3B active per token. The current sweet spot for single-GPU workstations.
- **Qwen3-VL** — the Qwen3 vision-language branch, available in several sizes on the [Qwen Hugging Face org](https://huggingface.co/Qwen). Pick the size that fits your GPU budget.

Both are distinct models in the same family — not aliases of each other. Either one works reliably once `GHOSTDESK_MODEL_SPACE=1000` is set.

If you prefer a hosted frontier model instead (Claude, GPT-4o, Gemini), set `GHOSTDESK_MODEL_SPACE=0` — they emit native screen pixels.

---

## Configuration

Every variable GhostDesk reads is namespaced under `GHOSTDESK_*`. Standard POSIX variables (`TZ`, `LANG`) are kept as-is so the existing Unix ecosystem keeps working.

### Secrets (required — container refuses to boot without them)

| Variable | Description |
|----------|-------------|
| `GHOSTDESK_AUTH_TOKEN` | Bearer token required on every MCP request. Generate with `openssl rand -hex 32`. |
| `GHOSTDESK_VNC_PASSWORD` | Password for wayvnc (username is `agent` in the prod image). Generate with `openssl rand -hex 16`. |

Both are plain environment variables — on Kubernetes wire them from a `Secret` via `valueFrom.secretKeyRef`; on Docker / compose inject them via `environment:` backed by Docker secrets or your secret manager. A front-door reverse proxy / identity-aware proxy in front of port 6080 is still recommended in production (the in-container VNC password is defense in depth) — see [Security](#security).

### Runtime knobs

| Variable | Default | Description |
|----------|---------|-------------|
| `GHOSTDESK_PORT` | `3000` | MCP server listening port |
| `GHOSTDESK_VNC_ADDRESS` | `127.0.0.1` | *(Read-only — hard-pinned.)* wayvnc is locked to loopback inside the container's netns; the VNC port is only reachable via the noVNC bridge on 6080. Override attempts are logged and ignored — see [SECURITY.md](SECURITY.md#transport-security). |
| `GHOSTDESK_TLS_CERT` | `/etc/ghostdesk/tls/server.crt` | Path to the TLS certificate. When the file exists, `websockify` and the MCP server auto-switch to `wss://` / `https://`. See [Security](#security). |
| `GHOSTDESK_TLS_KEY` | `/etc/ghostdesk/tls/server.key` | Path to the TLS private key (matching `GHOSTDESK_TLS_CERT`). |
| `GHOSTDESK_SCREEN_WIDTH` | `1280` | Virtual screen width in pixels |
| `GHOSTDESK_SCREEN_HEIGHT` | `1024` | Virtual screen height in pixels |
| `GHOSTDESK_MODEL_SPACE` | `1000` | LLM coordinate convention. `0` for frontier models that emit native screen pixels (Claude, GPT-4o, Gemini); `1000` for the Qwen vision family (Qwen3.5, Qwen3-VL, …) which emits a normalised 0-1000 space. See [Model requirements](#model-requirements) → *Coordinate space* for the full rationale. |
| `TZ` | `America/New_York` | IANA timezone (POSIX standard, e.g. `Europe/Paris`) |
| `LANG` | `en_US.UTF-8` | POSIX locale (e.g. `fr_FR.UTF-8`) |

---

## Security

GhostDesk ships hardened by default on the two axes where it is the product's own responsibility: **transport encryption** and **authentication**. Everything else (rate limiting, SSO, WAF, session recording, brute-force protection) is a reverse-proxy concern — GhostDesk is designed to run behind one, not directly on the internet.

### What the product guarantees

- **Operator-owned TLS, with an in-container fallback.** By default, `websockify` (port 6080) and the MCP server (port 3000) serve plain HTTP — TLS is expected to be terminated upstream (reverse proxy, cloud LB, devcontainer port forward). If the operator prefers in-container termination instead, mount a cert at `/etc/ghostdesk/tls/server.{crt,key}`; both services detect it at startup and switch to HTTPS / WSS automatically.
  ```yaml
  volumes:
    - /etc/letsencrypt/live/agent.example.com/fullchain.pem:/etc/ghostdesk/tls/server.crt:ro
    - /etc/letsencrypt/live/agent.example.com/privkey.pem:/etc/ghostdesk/tls/server.key:ro
  ```
- **wayvnc pinned to loopback, password auth inside the `wss://` envelope.** `wayvnc` is hard-pinned to `127.0.0.1:5900` inside the container's network namespace and is not reachable from outside — every browser session goes through the `websockify` bridge on port 6080. Under TLS, wayvnc is configured with `enable_auth=true` + `password=${GHOSTDESK_VNC_PASSWORD}` (no username) and advertises **RFB security type 2** (classic VNC Auth): noVNC 1.6 handles this natively and shows a single-password prompt in its overlay. The DES challenge/response used by RFB type 2 is cryptographically weak on its own — confidentiality on the wire is provided end-to-end by the `wss://` envelope on the websockify leg, and the VNC password acts as an authentication token carried inside that tunnel. This configuration is a deliberate compromise because noVNC 1.6 does not yet interoperate with wayvnc's stronger security types (VeNCrypt X509Plain / RSA-AES); GhostDesk ships wayvnc built from a pinned `master` commit that enables this path. `GHOSTDESK_VNC_PASSWORD` is mandatory. See [SECURITY.md](SECURITY.md#transport-security) for the full rationale.
- **Mandatory MCP authentication.** The MCP server refuses requests without a valid `Authorization: Bearer <GHOSTDESK_AUTH_TOKEN>` header.
- **Secrets as plain env vars, sourced from your secret store.** Both `GHOSTDESK_AUTH_TOKEN` and `GHOSTDESK_VNC_PASSWORD` are read from the process environment. On Kubernetes, wire them from a `Secret` via `valueFrom.secretKeyRef` (values never appear in `kubectl describe pod`); on Docker, inject via `environment:` backed by Docker secrets / Vault / AWS Secrets Manager. The container never bakes a secret into an image layer.

### What is explicitly *not* in scope

These belong to the operator's deployment topology, not to the container:

- **Rate limiting & brute-force protection** → reverse proxy (Traefik middleware, nginx `limit_req`, Cloudflare).
- **SSO / OIDC / MFA** → identity-aware proxy (oauth2-proxy, Cloudflare Access, Tailscale, Pomerium).
- **WAF / IP allow-listing** → edge.
- **Per-user identity / SSO on noVNC (port 6080)** → reverse proxy with OAuth2-proxy, Cloudflare Access, Tailscale ACLs, Pomerium, devcontainer port forward, etc. The in-container VNC password is a single shared credential — for per-user audit trail and revocation, gate port 6080 with an identity-aware proxy. See [SECURITY.md](SECURITY.md#the-novnc-deployment-contract).
- **Session recording & audit trail** → proxy access logs + wayvnc stderr shipped to your log collector.
- **Cert rotation & CA trust** → your PKI / cert-manager / Let's Encrypt automation.

This separation is deliberate and standard: the product handles crypto and authN, the infrastructure handles policy.

---

## Custom image

The `base` tag provides GhostDesk without any pre-installed GUI application — just the virtual desktop, VNC, and the MCP server. Use it to build your own image with only the tools you need:

```dockerfile
FROM ghcr.io/yv17labs/ghostdesk:base

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        chromium-browser \
        libreoffice-calc \
    && rm -rf /var/lib/apt/lists/*
```

```bash
docker build -t my-agent .
```

See the project's [Dockerfile](Dockerfile) for a complete example.

| Tag | Description |
|-----|-------------|
| `latest`, `X.Y.Z`, `X.Y` | Full image — includes Firefox, terminal, sudo |
| `base`, `base-X.Y.Z`, `base-X.Y` | Minimal image — no GUI app, meant to be extended |

---

## Tests

```bash
uv run pytest --cov
```

---

## License

AGPL-3.0 with Commons Clause — see [LICENSE](LICENSE).

Commercial use (resale, paid SaaS, etc.) requires written permission from the project owner.
