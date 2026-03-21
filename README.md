# GhostDesk

MCP server that gives LLM agents full control of a virtual Linux desktop — mouse, keyboard, screen capture, accessibility tree, clipboard, and shell. Runs inside Docker with human-like input simulation (Bézier mouse curves, variable typing delays) to bypass bot detection.

## Quick start

Open the project in VS Code → **Reopen in Container**. Then:

```bash
sudo supervisord -c /etc/supervisor/conf.d/ghostdesk.conf
```

This starts the virtual desktop (Xvfb + Openbox + VNC + noVNC) and the MCP server.

## Services

| Service | URL |
|---------|-----|
| MCP server | `http://localhost:3000/mcp` (Streamable HTTP) |
| VNC | `vnc://localhost:5900` (password: `changeme`) |
| noVNC | `http://localhost:6080/vnc.html` |

## Architecture

```
src/ghostdesk/
├── server.py              ← entry point, wiring only
├── instructions.py        ← LLM-facing tool documentation
├── roles.py               ← single source of truth for AT-SPI roles
├── _logging.py            ← logging setup
│
├── tools/
│   ├── devices/           ← mouse, keyboard, screen (xdotool/maim — human-like)
│   ├── accessibility/     ← AT-SPI read/interact/click (fast, no bot detection)
│   ├── shell/             ← exec, launch, wait
│   └── clipboard/         ← get/set via xclip
│
├── utils/                 ← cmd runner, humanizer, cursor overlay, window info
└── atspi/                 ← AT-SPI subprocess (runs under system Python with gi)
```

Two interaction channels by design: **devices** simulates physical input to stay undetected, **accessibility** uses AT-SPI for speed when stealth isn't needed.

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

## License

MIT — see [LICENSE](LICENSE).
