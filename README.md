<p align="center">
  <img src="assets/logo.png" alt="GhostDesk" width="200">
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
  <img src="demos/ghostdesk-amazon-sheets-automation.gif" alt="GhostDesk demo — Automated Amazon scraping to Google Sheets with real-time visualization" width="960">
</p>

---

## Why GhostDesk?

Most AI agents are trapped in text. They can call APIs and generate code, but they can't **use software**. GhostDesk changes that.

Connect any MCP-compatible LLM (Claude, GPT, Gemini...) and it gets a full Linux desktop with 11 tools to interact with **any application** — browsers, IDEs, office suites, terminals, legacy software, internal tools. No API needed. No integration required. If it has a UI, your agent can use it.

## What can your agent do with a full desktop?

Your agent gets its own Linux desktop. Here's what that unlocks:

### Agentic workflows — chain anything

```
"Go to the CRM, export last month's leads as CSV,
 open LibreOffice Calc, build a pivot table,
 take a screenshot of the chart, and email it to the team."
```

Your agent opens the browser, logs in, downloads the file, switches to another app, processes the data, captures the result, and sends it — autonomously, across multiple applications, in one conversation.

### Browse the web like a human

```
"Search for competitors on Google, open the first 5 results,
 extract pricing from each page, and summarize in a spreadsheet."
```

No Selenium. No CSS selectors. No Puppeteer scripts that break every week. The agent looks at the screen, clicks what it sees, fills forms naturally — with human-like mouse movement that bypasses bot detection.

### Operate any software — no API required

```
"Open the legacy inventory app, search for product #4521,
 update the stock count to 150, and confirm the change."
```

That old Java app with no API? That internal admin panel from 2010? A Windows app running in Wine? If it renders pixels on screen, your agent can operate it.

### Data extraction at scale

```
"Open the analytics dashboard, read the KPI table,
 scroll down to the revenue chart, take a screenshot,
 then export the raw data."
```

The agent takes screenshots, reads the screen visually, and extracts what it needs — works on any application, any UI framework, any language.

### QA & UI testing with evidence

```
"Navigate the signup flow, try invalid emails, empty fields,
 and SQL injection in every input. Screenshot each error state."
```

Your agent becomes a QA engineer — it clicks every button, fills every form, tests every edge case, and brings back screenshots as proof.

### Unattended automation — runs 24/7

```
"Every morning: log into the supplier portal, download
 the latest price list, compare with yesterday's, and
 flag any changes above 5%."
```

Runs headless in Docker. No physical screen. No human babysitting. Schedule your agent to handle repetitive desktop tasks while you sleep.

### Multi-app orchestration

```
"Open VS Code, create a new Python file, write a script
 that calls our API, run it in the terminal, debug if it fails,
 then commit and push to GitHub."
```

Your agent isn't limited to one app. It can switch between browser, terminal, IDE, file manager, email client — just like a human switching windows on their desktop.

## Key features

| | Feature | Why it matters |
|---|---|---|
| **📸** | **Screenshots** | Full or regional captures with cursor overlay — the agent sees exactly what a human would see |
| **🖱️** | **Human-like input** | Bézier mouse curves, variable typing speed, micro-jitter — bypasses bot detection |
| **📋** | **Clipboard** | Read & write the clipboard — paste long text instantly |
| **⌨️** | **Keyboard control** | Type text, press hotkeys, keyboard shortcuts — full keyboard access |
| **🖥️** | **Shell access** | Run any command, launch any app, capture stdout/stderr |
| **🐳** | **Sandboxed** | Runs in Docker — isolated, reproducible, safe |
| **👀** | **Live view** | Watch your agent work in real-time via VNC or browser (noVNC) |

## 11 tools at your agent's fingertips

### Screen
| Tool | Description |
|------|-------------|
| `screenshot` | Capture the screen as an image (full or region). Use `annotate=True` to overlay detected elements with coordinate labels |
| `inspect` | Read the screen as structured JSON — cursor, windows, and all visible UI elements with click coordinates (no image) |

### Mouse & keyboard
| Tool | Description |
|------|-------------|
| `mouse_click` | Click at coordinates |
| `mouse_double_click` | Double-click at coordinates |
| `mouse_drag` | Drag from one position to another |
| `mouse_scroll` | Scroll in any direction (up/down/left/right) |
| `type_text` | Type with realistic per-character delays |
| `press_key` | Press keys or combos (`ctrl+c`, `alt+F4`, `Return`...) |

### System
| Tool | Description |
|------|-------------|
| `launch` | Start GUI applications |
| `get_clipboard` | Read clipboard contents |
| `set_clipboard` | Write to clipboard |

## Quick start

### 1. Run the container

```bash
docker run -d --name ghostdesk \
  -p 3000:3000 \
  -p 5900:5900 \
  -p 6080:6080 \
  ghcr.io/yv17labs/ghostdesk:latest
```

That's it. The virtual desktop, MCP server, and VNC are all running inside an isolated container. Your agent gets a full Linux desktop — your host machine stays untouched.

### 2. Connect your AI

GhostDesk works with any MCP-compatible client. Add it to your config:

**Claude Desktop / Claude Code** (Streamable HTTP)
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

**ChatGPT, Gemini, or any LLM with MCP support** — same config, just point to `http://localhost:3000/mcp`.

### 3. Watch your agent work

Open `http://localhost:6080/vnc.html` in your browser to see the virtual desktop in real time.

| Service | URL |
|---------|-----|
| MCP server | `http://localhost:3000/mcp` |
| noVNC (browser) | `http://localhost:6080/vnc.html` |
| VNC | `vnc://localhost:5900` (password: `changeme`) |

## Demos

See GhostDesk in action:

| Demo | Description |
|------|-------------|
| [Google Sheets Automation](demos/ghostdesk-sheets-automation.gif) | AI agent autonomously populates a spreadsheet with AI startup funding data, formats headers, and creates a 3D bar chart |
| [Amazon Scraper to Google Sheets](demos/amazon-scraper-to-sheets.gif) | AI agent scrapes Amazon laptops, extracts product data, populates Google Sheets, and visualizes with charts |
| [Flight Search & Comparison](demos/ghostdesk-flight-search.gif) | AI agent searches Google Flights for Paris CDG → New York JFK, compares prices, and builds a chart in LibreOffice Calc |
| [Wikipedia Research](demos/ghostdesk-wikipedia.gif) | AI agent browsing Wikipedia, reading articles, and extracting information |

## How it works

GhostDesk runs a virtual Linux desktop inside Docker and exposes it as an MCP server. Your LLM agent connects and gets two ways to perceive the screen:

- **Vision mode** (`screenshot`) — the agent takes a screenshot and interprets it visually, like a human looking at their monitor. Best for large models with strong vision (Claude, GPT-4, Gemini).
- **Text mode** (`inspect`) — the agent receives structured JSON with every visible UI element and its click coordinates. No image interpretation needed. Best for smaller models or text-only workflows.

Then the agent acts — clicks, types, scrolls, or runs commands using human-like input simulation (Bézier mouse curves, variable typing delays, micro-jitter) — and verifies the result.

This approach works with **any application** — web apps, native apps, legacy software, Canvas, WebGL. If it renders pixels, the agent can use it.

## Model Requirements

GhostDesk works best with models that have both **vision and tool use**.

### Large models (Claude, GPT-4, Gemini...)

No special setup needed. These models are excellent at interpreting screenshots and estimating coordinates. The MCP server provides built-in instructions that are enough.

### Small models (local / edge)

Small vision models need more guidance to interact with the desktop reliably. We provide a dedicated prompt optimized for them: **[SYSTEM_PROMPT_SMALL_MODEL.md](SYSTEM_PROMPT_SMALL_MODEL.md)**.

It focuses on:
- **Always using `screenshot(annotate=True)`** — small models struggle to estimate coordinates from raw screenshots, so the annotated overlay with bounding boxes and `(x,y)` labels is essential.
- **A strict LOOK → ACT → VERIFY loop** — one action at a time, always verify with a fresh screenshot.
- **Concrete patterns** — copy-paste workflows for common tasks (click, type, scroll, close popups...).

**Best small model tested to date:** [Qwen3.5-35B-A3B](https://huggingface.co/Qwen/Qwen3.5-35B-A3B) — a 35B MoE model with only 3B active parameters. Recommended as a starting point for local deployments. Below this size, results are possible but unreliable.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `SCREEN_WIDTH` | `1280` | Virtual screen width |
| `SCREEN_HEIGHT` | `1024` | Virtual screen height |
| `VNC_PASSWORD` | `changeme` | VNC access password |
| `PORT` | `3000` | MCP server port |
| `TZ` | `UTC` | Timezone (e.g. `Europe/Paris`, `America/Toronto`) |
| `LOCALE` | `en_US.utf8` | System locale (e.g. `fr_FR.utf8`, `fr_CA.utf8`) |

## Tests

```bash
uv run pytest --cov
```

## License

AGPL-3.0 with Commons Clause — see [LICENSE](LICENSE).

Commercial use (resale, paid SaaS, etc.) requires written permission from the project owner.
