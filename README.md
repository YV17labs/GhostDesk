# GhostDesk

MCP server that gives LLM agents full control of a virtual Linux desktop inside Docker — mouse, keyboard, screenshots, browser, and shell commands. Features human-like mouse movement (Bézier curves) and typing, with a visible cursor overlay on every screenshot.

## Quick start

### Development (Dev Container)

Open the project in VS Code and use **"Reopen in Container"**. Once inside, start the virtual desktop and the MCP server in two separate terminals:

```bash
# Terminal 1 — Start the virtual desktop (Xvfb + Openbox + VNC + noVNC)
sudo supervisord -c /etc/supervisor/conf.d/ghostdesk.conf
```

```bash
# Terminal 2 — Start the MCP server
uv run ghostdesk
```

### Services

| Service | URL | Description |
|---------|-----|-------------|
| MCP server | `http://localhost:3000/mcp` | Streamable HTTP transport for LLM agents |
| VNC | `vnc://localhost:5900` | Direct VNC access (password: `changeme`) |
| noVNC | `http://localhost:6080/vnc.html` | Browser-based desktop viewer (no client needed) |

## MCP tools

### Vision
- `screenshot(x?, y?, width?, height?)` — full-screen or region capture. Returns the image with a **red crosshair** at the current cursor position, plus metadata: cursor coordinates, active window title, and list of visible windows with geometry.
- `get_screen_size()` — returns `{width, height}`

### Mouse
All mouse tools accept `humanize` (default `True`): when enabled, the cursor follows a **Bézier curve** with easing and micro-jitter instead of teleporting.

- `mouse_move(x, y, humanize?)` — move cursor
- `mouse_click(x, y, button?, humanize?)` — click (left/middle/right)
- `mouse_double_click(x, y, button?, humanize?)` — double-click
- `mouse_drag(from_x, from_y, to_x, to_y, button?, humanize?)` — drag
- `mouse_scroll(x, y, direction?, amount?, humanize?)` — scroll (up/down/left/right)

### Keyboard
- `type_text(text, delay_ms?, humanize?)` — type characters. With `humanize=True` (default), varies delay per character: faster mid-word, slower after spaces and punctuation.
- `press_key(keys)` — key combo (e.g. `ctrl+c`, `Return`, `alt+F4`)

### Utilities
- `wait(milliseconds)` — pause before next action
- `run_command(command, timeout_seconds?)` — execute shell command
- `open_url(url, new_tab?)` — open URL in Firefox. Reuses the existing browser window via the address bar (Ctrl+L) if Firefox is already running. Set `new_tab=True` to open in a new tab.

## Application Startup & Readiness

### Starting Applications
`launch(command)` spawns the application **immediately and returns**. It does NOT wait for the application to become responsive.

### Verifying Readiness
**DO NOT use arbitrary `wait()` calls to check if an application is ready.** This is unreliable across different system speeds and application complexities.

Instead, **actively verify readiness**:
1. After `launch()`, immediately call `list_elements()` or `screenshot()`
2. If the command succeeds and returns data → application is ready
3. If the command times out or fails → application not ready yet; wait briefly (100-500ms) and retry

**Good pattern:**
```
launch("firefox https://example.com")
list_elements()  // If this succeeds, app is ready; proceed with interactions
```

**Bad pattern:**
```
launch("firefox https://example.com")
wait(3000)  // ✗ Arbitrary guess, unreliable across different systems
list_elements()
```

### When to Use `wait()`
Use `wait()` **only for**:
- UI animations or transitions (e.g., fade-in, slide-out, modal appearance)
- Expected application delays (e.g., file save, network request)

**NOT for verifying application startup.** Hardcoded waits break across network speeds, system load, and application complexity. Active verification is always more reliable.

## Human-like behavior

GhostDesk emulates human interaction patterns to avoid bot detection:

- **Mouse movement**: Cubic Bézier curves with sinusoidal easing (slow start → fast middle → slow end) and ±2px jitter on the target.
- **Typing**: Gaussian-distributed delays per character — faster mid-word, longer pauses after spaces and punctuation.
- **Cursor visibility**: Every screenshot includes a red crosshair overlay so the LLM always knows where the cursor is.

All humanization can be disabled per-call with `humanize=False` for maximum speed.

## Configuration

Environment variables (see `.env.example`):

| Variable | Default | Description |
|----------|---------|-------------|
| `SCREEN_WIDTH` | `1280` | Virtual screen width |
| `SCREEN_HEIGHT` | `800` | Virtual screen height |
| `SCREEN_DEPTH` | `24` | Color depth |
| `VNC_PASSWORD` | `changeme` | VNC access password |
| `PORT` | `3000` | MCP server port |

## Connect an LLM agent

Point your MCP-compatible client at `http://localhost:3000/mcp` using the Streamable HTTP transport.

## License

MIT — see [LICENSE](LICENSE).
