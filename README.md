<p align="center">
  <img src="https://img.shields.io/badge/MCP-compatible-blueviolet?style=for-the-badge" alt="MCP Compatible">
  <img src="https://img.shields.io/badge/python-3.12+-blue?style=for-the-badge&logo=python&logoColor=white" alt="Python 3.12+">
  <img src="https://img.shields.io/badge/license-MIT-green?style=for-the-badge" alt="MIT License">
  <img src="https://img.shields.io/badge/coverage-97%25-brightgreen?style=for-the-badge" alt="Coverage 97%">
  <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Docker-orange?style=for-the-badge&logo=docker&logoColor=white" alt="Platform">
</p>

<h1 align="center">GhostDesk</h1>

<p align="center">
  <strong>Give your AI agent eyes, hands, and a full Linux desktop.</strong><br>
  An MCP server that lets LLM agents see the screen, move the mouse, type on the keyboard, read UI elements, fill forms, launch apps, and run shell commands — all inside a sandboxed virtual desktop.
</p>

<p align="center">
  <em>If a human can do it on a desktop, your agent can too.</em>
</p>

---

## Why GhostDesk?

Most AI agents are trapped in text. They can call APIs and generate code, but they can't **use software**. GhostDesk changes that.

Connect any MCP-compatible LLM (Claude, GPT, Gemini...) and it gets a full Linux desktop with 25+ tools to interact with **any application** — browsers, IDEs, office suites, terminals, legacy software, internal tools. No API needed. No integration required. If it has a UI, your agent can use it.

## What can you do with it?

**Web automation without brittle selectors**
> Your agent sees the page like a human does. It reads buttons, fills forms, clicks links — no CSS selectors, no Selenium scripts, no breaking when the UI changes.

**Automate legacy & internal tools**
> That old Java app with no API? That internal admin panel? GhostDesk doesn't care. If it renders on screen, your agent can operate it.

**End-to-end workflow automation**
> Chain actions across multiple applications: pull data from a browser, paste it into a spreadsheet, generate a report, email it — all in one conversation.

**Data extraction from any UI**
> Scrape tables, read dashboards, extract form data. The accessibility engine returns structured data, not raw pixels.

**QA & UI testing**
> Let your agent navigate your app, fill every form, click every button, and report what's broken — with screenshots as evidence.

**Unattended desktop tasks**
> Schedule your agent to log into portals, download reports, update records, or monitor dashboards. Runs headless in Docker — no physical screen needed.

## Key features

| | Feature | Why it matters |
|---|---|---|
| **👁️** | **Accessibility engine** | Reads UI elements semantically (buttons, inputs, labels, tables) — fast, structured, zero vision cost |
| **🖱️** | **Human-like input** | Bézier mouse curves, variable typing speed, micro-jitter — bypasses bot detection |
| **📸** | **Screenshots** | Full or regional captures with cursor overlay — for when your agent needs to *see* |
| **📋** | **Clipboard** | Read & write the clipboard — paste long text instantly |
| **⌨️** | **Keyboard control** | Type text, press hotkeys, keyboard shortcuts — full keyboard access |
| **🖥️** | **Shell access** | Run any command, launch any app, capture stdout/stderr |
| **📊** | **Table extraction** | Pull structured table data (headers + rows) from any application |
| **🔍** | **Smart element detection** | Wait for elements to appear, scroll them into view, inspect their state |
| **🐳** | **Sandboxed** | Runs in Docker — isolated, reproducible, safe |
| **👀** | **Live view** | Watch your agent work in real-time via VNC or browser (noVNC) |

## 25+ tools at your agent's fingertips

### Read & understand the screen
| Tool | Description |
|------|-------------|
| `read_screen()` | Get all visible UI elements — names, roles, states — in reading order |
| `get_element_details()` | Inspect any element: its value, actions, children, position |
| `read_table()` | Extract structured table data as headers + rows |
| `screenshot()` | Capture the screen (full or region) with cursor position overlay |
| `get_screen_size()` | Get current screen resolution |

### Interact with the UI
| Tool | Description |
|------|-------------|
| `click_element()` | Find an element by name and click it |
| `set_value()` | Set text, numbers, or slider values on form fields |
| `focus_element()` | Give keyboard focus to any element |
| `scroll_to_element()` | Scroll an off-screen element into view |
| `wait_for_element()` | Wait until an element appears (with configurable timeout) |

### Mouse & keyboard
| Tool | Description |
|------|-------------|
| `mouse_move()` | Move the cursor with natural Bézier trajectories |
| `mouse_click()` | Click at coordinates (left / middle / right) |
| `mouse_double_click()` | Double-click at coordinates |
| `mouse_drag()` | Drag with human-like movement |
| `mouse_scroll()` | Scroll in any direction |
| `type_text()` | Type with realistic per-character delays |
| `press_key()` | Press keys or combos (`ctrl+c`, `alt+F4`, `Return`...) |

### System
| Tool | Description |
|------|-------------|
| `exec()` | Run shell commands with stdout/stderr capture |
| `launch()` | Start GUI applications |
| `wait()` | Pause execution |
| `get_clipboard()` | Read clipboard contents |
| `set_clipboard()` | Write to clipboard |

## Quick start

### 1. Open in VS Code Dev Container

Clone the repo, open it in VS Code, and use **"Reopen in Container"**.

### 2. Start the desktop

```bash
sudo supervisord -c /etc/supervisor/conf.d/ghostdesk.conf
```

This boots the virtual desktop (Xvfb + Openbox), VNC server, and the MCP server.

### 3. Connect your agent

Add GhostDesk to your MCP client config:

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

### 4. Watch your agent work

Open your browser and go to `http://localhost:6080/vnc.html` to see the virtual desktop in real time.

| Service | URL |
|---------|-----|
| MCP server | `http://localhost:3000/mcp` |
| noVNC (browser) | `http://localhost:6080/vnc.html` |
| VNC | `vnc://localhost:5900` (password: `changeme`) |

## How it works

GhostDesk uses **two interaction channels** that your agent switches between automatically:

**Accessibility channel** — Uses Linux's AT-SPI (the same API screen readers use) to read and interact with UI elements. Fast, structured, no screenshots needed. Perfect for forms, buttons, menus, and tables.

**Devices channel** — Simulates real mouse and keyboard input with human-like behavior: Bézier curves for mouse movement, variable typing delays, micro-jitter on click targets. Indistinguishable from a real user. Used when accessibility isn't enough — canvas apps, CAPTCHAs, visual verification.

Your agent starts with accessibility (fast & cheap), and falls back to devices (stealth & visual) only when needed.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `SCREEN_WIDTH` | `1280` | Virtual screen width |
| `SCREEN_HEIGHT` | `800` | Virtual screen height |
| `SCREEN_DEPTH` | `24` | Color depth |
| `VNC_PASSWORD` | `changeme` | VNC access password |
| `PORT` | `3000` | MCP server port |

## Tests

```bash
uv run pytest --cov
```

183 tests — 97% coverage.

## License

MIT — see [LICENSE](LICENSE).
