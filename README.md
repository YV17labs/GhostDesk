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

## Table of contents

- [Why GhostDesk?](#why-ghostdesk)
- [How it works](#how-it-works)
- [Quick start](#quick-start)
- [Secure local run (TLS + auth)](#secure-local-run-tls--auth)
- [Tools](#tools)
- [Model requirements](#model-requirements)
- [From one agent to a workforce](#from-one-agent-to-a-workforce)
- [Configuration](#configuration)
- [Security](#security)
- [Troubleshooting](#troubleshooting)
- [Custom image](#custom-image)
- [License](#license)

---

## Why GhostDesk?

Browser automation tools (Playwright, Puppeteer, Selenium…) were built for human test engineers driving a browser with selectors. They do one thing, and they do it well — inside the browser.

GhostDesk is built from the other end: for **AI agents**, driving **everything a desktop runs**. Browsers, native apps, IDEs, terminals, office suites, legacy software, internal tools. If it renders pixels on screen, your agent can see it and use it — in one conversation, across many applications, without a line of glue code.

You don't write selectors. You write a prompt:

> *"Open the CRM, export last month's leads as CSV, open LibreOffice Calc, build a pivot table, screenshot the chart, and email it to the team."*

The agent opens the browser, logs in, downloads the file, switches to LibreOffice, processes the data, captures the result, composes the email, sends it. One prompt, multiple apps, fully autonomous — no glue code, no per-site scraper, no brittle selector chain.

That is what *agents using a desktop* looks like.

### Runs on models you can actually host

Desktop control needs to be **fast** — an agent that takes twelve seconds to decide where to click is unusable. GhostDesk is tuned so that vision-language models from the Qwen family running on a single workstation GPU are a first-class target, not an afterthought. No API bill, no screenshots of your desktop leaving your network.

Frontier models (Claude, GPT-4o, Gemini) work too and remain the smoothest path — but they are not the bar. See [Model requirements](#model-requirements) for the supported stacks and the one coordinate-space setting that matters.

---

## How it works

GhostDesk runs a virtual Linux desktop inside Docker and exposes it as an MCP server. Your agent gets a sandboxed desktop with a taskbar, clock, and pre-installed applications — equivalent to what a human sees on their screen.

The agent perceives the screen by calling `screen_shot()`, which captures the full desktop at native resolution and returns it as WebP (or PNG). An optional `region=` argument can crop to a sub-rectangle when the agent explicitly wants to narrow its focus.

This works with **any application** — web apps, native apps, legacy software, Canvas, WebGL.

---

## Quick start

### 1. Run the container

One command, plain HTTP, no password. Fine for kicking the tires on a laptop you trust — **not fit for anything beyond that**. Ready to harden it? Jump to [Secure local run](#secure-local-run-tls--auth).

```bash
docker run -d --name ghostdesk-demo \
  --shm-size 2g \
  -p 3000:3000 \
  -p 6080:6080 \
  ghcr.io/yv17labs/ghostdesk:latest
```

The `latest` image ships with **Firefox**, the **foot** terminal, **mousepad** (text editor), **galculator**, and passwordless `sudo` for the `agent` user — enough to demo a browsing + note-taking workflow out of the box. Need a different app set? Build your own on top of `base` — see [Custom image](#custom-image).

The container boots in the dev posture: plain HTTP on both ports, every auth gate disarmed on purpose. You'll see warnings in the logs reminding you of that — they go away once you follow the secured path below.

### 2. Connect your AI

GhostDesk speaks [MCP](https://modelcontextprotocol.io/) over the Streamable HTTP transport — any MCP-compatible client can drive it. Point your client at `http://localhost:3000/mcp`:

**Claude Desktop / Claude Code**
```json
{
  "mcpServers": {
    "ghostdesk": {
      "type": "http",
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

**Any other MCP-compatible client** — same URL, no headers, no auth. That's the whole demo posture.

> **Drop in the recommended system prompt.** [`SYSTEM_PROMPT.md`](SYSTEM_PROMPT.md) gives your agent a battle-tested baseline — keyboard first, see/act/confirm, clear popups, scroll-to-read. It measurably improves reliability across both frontier and self-hosted models. Copy it into your agent's system prompt before the first run.

### 3. Watch your agent work

Open `http://localhost:6080/` in your browser to see the virtual desktop in real time. No password prompt — the dev posture skips it.

| Service | URL |
|---------|-----|
| MCP server | `http://localhost:3000/mcp` |
| noVNC (browser) | `http://localhost:6080/` |

Give your agent a first prompt to confirm the wiring is right:

> *"Take a screenshot of the desktop, list the installed applications, then open Firefox and go to wikipedia.org."*

You should see Firefox launch in the noVNC tab, the URL bar fill in, and the page load — all under your agent's control.

### 4. When you're done

```bash
docker stop ghostdesk-demo && docker rm ghostdesk-demo
```

The demo run creates no named volume, so this leaves nothing behind.

---

## Secure local run (TLS + auth)

The Quick start above drops every gate so you can kick the tires in thirty seconds. The moment you want to expose this to anything beyond your own laptop — another machine on your LAN, a devcontainer port-forward on an untrusted network, a teammate's browser — flip to the secured posture: real TLS + bearer-token auth on MCP + password prompt on noVNC.

GhostDesk couples **TLS and auth**: mount a cert and you get `wss://` + bearer-token on MCP + a single-password prompt on noVNC (see [Security](#security) → *Auth ≡ TLS*). [`mkcert`](https://github.com/FiloSottile/mkcert) issues a browser-trusted cert for `localhost` in two commands:

```bash
# Issue a locally-trusted cert (first time only — installs a local CA in your trust store)
mkcert -install
mkdir -p tls
mkcert -cert-file tls/server.crt -key-file tls/server.key localhost 127.0.0.1 ::1

# Generate the MCP and VNC secrets
export GHOSTDESK_AUTH_TOKEN=$(openssl rand -hex 32)
export GHOSTDESK_VNC_PASSWORD=$(openssl rand -hex 16)
```

Pick a container name that matches the agent's role — `sales-agent`, `research-agent`, `accounting-agent`… Below we use `my-agent` as a placeholder; replace it everywhere in the command.

```bash
# Run the container — cert mounted, TLS + auth enabled everywhere
docker run -d --name ghostdesk-my-agent \
  --restart unless-stopped \
  --cap-add SYS_ADMIN \
  --shm-size 2g \
  -p 3000:3000 \
  -p 6080:6080 \
  -v ghostdesk-my-agent-home:/home/agent \
  -v "$PWD/tls/server.crt:/etc/ghostdesk/tls/server.crt:ro" \
  -v "$PWD/tls/server.key:/etc/ghostdesk/tls/server.key:ro" \
  -e GHOSTDESK_AUTH_TOKEN \
  -e GHOSTDESK_VNC_PASSWORD \
  -e TZ=America/New_York \
  -e LANG=en_US.UTF-8 \
  ghcr.io/yv17labs/ghostdesk:latest

echo "MCP token:    $GHOSTDESK_AUTH_TOKEN"
echo "VNC password: $GHOSTDESK_VNC_PASSWORD"
```

Once the container is up, update your MCP client config — same shape as the demo, now over `https://` with a bearer token:

**Claude Desktop / Claude Code**
```json
{
  "mcpServers": {
    "ghostdesk": {
      "type": "http",
      "url": "https://localhost:3000/mcp",
      "headers": {
        "Authorization": "Bearer <paste $GHOSTDESK_AUTH_TOKEN here>"
      }
    }
  }
}
```

**Any other MCP-compatible client** — same URL, plus an `Authorization: Bearer <token>` header in whatever form your client accepts.

Then open `https://localhost:6080/` in your browser — the `mkcert` CA installed by `mkcert -install` is already in your trust store, so the browser accepts the cert with no warning. noVNC will prompt for `$GHOSTDESK_VNC_PASSWORD`.

> **Going to production?** Swap the `mkcert` leaf for a real cert, source both secrets from your secret manager, and front port 6080 with an identity-aware proxy — [SECURITY.md](SECURITY.md) has the full contract.

> **`--cap-add SYS_ADMIN`** — Required by Electron apps (VS Code, Slack, etc.) and other applications that need Linux user namespaces to run their sandbox. Safe to remove if you don't need them.

The named volume persists the agent's home directory across restarts — browser passwords, bookmarks, cookies, downloads, and desktop preferences are all preserved. On the first run, Docker automatically seeds the volume with the default configuration from the image.

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

## From one agent to a workforce

Each GhostDesk instance is a container. Spin up one, ten, or a hundred — each agent gets its own isolated desktop, its own apps, its own role. Think of it as hiring a team of digital employees, each with their own workstation.

### Scale horizontally

```yaml
# docker-compose.yml — 3 specialized agents, one command
#
# Prerequisites: the TLS cert + key at ./tls and the two secrets
# (GHOSTDESK_AUTH_TOKEN, GHOSTDESK_VNC_PASSWORD) in your environment or a
# .env file. Generate both exactly as shown in the Secure local run
# section above. See SECURITY.md for the production secret-handling
# contract.

x-ghostdesk-defaults: &ghostdesk-defaults
  image: ghcr.io/yv17labs/ghostdesk:latest
  restart: unless-stopped
  cap_add: [SYS_ADMIN]
  shm_size: 2g
  environment:
    - GHOSTDESK_AUTH_TOKEN
    - GHOSTDESK_VNC_PASSWORD
    - TZ=America/New_York
    - LANG=en_US.UTF-8

services:
  sales-agent:
    <<: *ghostdesk-defaults
    container_name: ghostdesk-sales-agent
    ports: ["3001:3000", "6081:6080"]
    volumes:
      - ghostdesk-sales-agent-home:/home/agent
      - ./tls/server.crt:/etc/ghostdesk/tls/server.crt:ro
      - ./tls/server.key:/etc/ghostdesk/tls/server.key:ro

  research-agent:
    <<: *ghostdesk-defaults
    container_name: ghostdesk-research-agent
    ports: ["3002:3000", "6082:6080"]
    volumes:
      - ghostdesk-research-agent-home:/home/agent
      - ./tls/server.crt:/etc/ghostdesk/tls/server.crt:ro
      - ./tls/server.key:/etc/ghostdesk/tls/server.key:ro

  accounting-agent:
    <<: *ghostdesk-defaults
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

## Configuration

Every variable GhostDesk reads is namespaced under `GHOSTDESK_*`. Standard POSIX variables (`TZ`, `LANG`) are kept as-is so the existing Unix ecosystem keeps working.

### Secrets (required — container refuses to boot without them)

| Variable | Description |
|----------|-------------|
| `GHOSTDESK_AUTH_TOKEN` | Bearer token required on every MCP request. Generate with `openssl rand -hex 32`. |
| `GHOSTDESK_VNC_PASSWORD` | Password for wayvnc (username is `agent` in the prod image). Generate with `openssl rand -hex 16`. |

Both are plain environment variables. Wire them from your secret store (`secretKeyRef` on Kubernetes, Docker secrets / Vault / AWS SM on compose) — see [SECURITY.md](SECURITY.md#secrets-handling--rotation) for the full contract.

### Runtime knobs

| Variable | Default | Description |
|----------|---------|-------------|
| `GHOSTDESK_PORT` | `3000` | MCP server listening port |
| `GHOSTDESK_TLS_CERT` | `/etc/ghostdesk/tls/server.crt` | Path to the TLS certificate. When the file exists, `websockify` and the MCP server auto-switch to `wss://` / `https://`. See [Security](#security). |
| `GHOSTDESK_TLS_KEY` | `/etc/ghostdesk/tls/server.key` | Path to the TLS private key (matching `GHOSTDESK_TLS_CERT`). |
| `GHOSTDESK_SCREEN_WIDTH` | `1280` | Virtual screen width in pixels |
| `GHOSTDESK_SCREEN_HEIGHT` | `1024` | Virtual screen height in pixels |
| `GHOSTDESK_MODEL_SPACE` | `1000` | LLM coordinate convention — `0` for frontier models, `1000` for the Qwen vision family. Full rationale in [Model requirements](#model-requirements) → *Coordinate space*. |
| `TZ` | `America/New_York` | IANA timezone (POSIX standard, e.g. `Europe/Paris`) |
| `LANG` | `en_US.UTF-8` | POSIX locale (e.g. `fr_FR.UTF-8`) |

### Pinned values (not configurable)

| Variable | Value | Rationale |
|----------|-------|-----------|
| `GHOSTDESK_VNC_ADDRESS` | `127.0.0.1` | wayvnc is locked to loopback inside the container's netns; the VNC port is only reachable via the noVNC bridge on 6080. Override attempts are logged and ignored — see [SECURITY.md](SECURITY.md#transport-security). |

---

## Security

GhostDesk owns two things: **transport encryption** and **authentication**. Everything else (rate limiting, SSO, WAF, session recording, brute-force protection, per-user identity on noVNC) is a reverse-proxy concern — the container is designed to run behind one, not directly on the internet.

The full threat model, the *Auth ≡ TLS* posture switch, the wayvnc RFB-type-2-inside-`wss://` rationale, the secrets handling contract, and the exhaustive in-scope / out-of-scope table all live in **[SECURITY.md](SECURITY.md)** — single source of truth. Start there before deploying to anything you don't fully trust.

Reporting a vulnerability? Use GitHub's [private security advisory](../../security/advisories) — see [SECURITY.md § Reporting](SECURITY.md#reporting-security-vulnerabilities).

---

## Troubleshooting

### My agent's clicks land off-target by a huge margin

Almost always a `GHOSTDESK_MODEL_SPACE` mismatch. Frontier models (Claude, GPT-4o, Gemini) need `0`; the Qwen vision family needs `1000`. Full rationale in [Model requirements](#model-requirements) → *Coordinate space*.

### The container refuses to start with a secrets error

The prod posture (cert mounted) **requires** both `GHOSTDESK_AUTH_TOKEN` and `GHOSTDESK_VNC_PASSWORD` to be set — GhostDesk refuses to boot without them on purpose, to prevent an unauthenticated prod container. Generate them as shown in [Secure local run](#secure-local-run-tls--auth) and pass them with `-e`. The demo posture (no cert) has no such requirement.

### noVNC shows a black screen or the desktop renders with graphical glitches

You're probably short on shared memory. Browsers and other GPU-accelerated apps inside the container need a reasonable `/dev/shm` — `--shm-size 2g` is the baseline in every example and should not be trimmed. If you already have `--shm-size 2g`, check the container logs for wayvnc or compositor errors.

### Firefox / Electron apps fail to launch or crash immediately

Electron-based apps (VS Code, Slack, Discord…) need Linux user namespaces for their sandbox. Add `--cap-add SYS_ADMIN` to your `docker run` (already present in the Secure local run example). Firefox itself works without it.

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
| `latest`, `X.Y.Z`, `X.Y` | Full image — Firefox, foot terminal, mousepad, galculator, passwordless sudo |
| `base`, `base-X.Y.Z`, `base-X.Y` | Minimal image — no GUI app, meant to be extended |

---

## License

**AGPL-3.0 with Commons Clause** — see [LICENSE](LICENSE) for the authoritative terms.

**What this means in practice** *(informal summary — not legal advice; the LICENSE file governs)*:

- The **AGPL-3.0** side means any modifications you make and run as a network service must be shared under the same license. Self-hosting and modifying GhostDesk for your own use is fine; making those modifications available over a network to users generally triggers the source-disclosure obligation.
- The **Commons Clause** side restricts commercial resale: you cannot sell GhostDesk itself, or sell a product whose value derives substantially from GhostDesk, without a separate agreement.

**Commercial use** (resale, paid SaaS built on GhostDesk, rebranded hosting, etc.) requires written permission from the project owner. If you're unsure whether your use case fits, open an issue or contact the maintainers before deploying.
