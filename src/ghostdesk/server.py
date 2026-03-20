# Copyright (c) 2026 YV17 — MIT License
"""GhostDesk — MCP server entry point."""

import logging
import os
import time

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools import keyboard, mouse, screenshot, shell

logger = logging.getLogger("ghostdesk")

_INSTRUCTIONS = """
You control a virtual Linux desktop (X11, display :99, resolution 1280×800).

## Tools
- exec(command) — run a shell command, returns stdout/stderr/returncode.
- launch(command) — launch a GUI app (fire-and-forget), returns immediately.
- screenshot() — capture the screen with a red crosshair showing cursor position.
- mouse_click(x, y) — click on a screen element.
- type_text(text) — type in the focused field.
- press_key(keys) — press a key combination.
- wait(milliseconds) — pause between actions.

## Workflow
1. launch("firefox https://...") — open a website in Firefox
2. wait(3000) — let the page load
3. screenshot() — see the screen
4. mouse_click(x, y) — click on the element you want
5. screenshot() — verify the result
6. type_text("...") — type in the focused field
7. screenshot() — verify

## Rules
- Use launch() for GUI apps (firefox, gedit, vlc...). Use exec() for CLI commands.
- After every action, take a screenshot to verify the effect.
- If a click has no visible effect, do not retry — scroll or reassess.
- Install software with exec("sudo apt-get install -y <package>").

## Humanization
Mouse and keyboard tools accept `humanize` (default True).
Set humanize=False when you need speed over stealth.

## Errors
If you receive "Session not found": stop and inform the user to reconnect.
"""

port = int(os.environ.get("PORT", "3000"))
mcp = FastMCP("ghostdesk", instructions=_INSTRUCTIONS, host="0.0.0.0", port=port)

# Register all tool modules
screenshot.register(mcp)
mouse.register(mcp)
keyboard.register(mcp)
shell.register(mcp)

# Wrap call_tool to log every tool invocation with name, args, and duration.
_original_call_tool = mcp.call_tool


async def _logged_call_tool(name: str, arguments: dict) -> object:
    args_summary = ", ".join(f"{k}={v!r}" for k, v in arguments.items())
    t0 = time.monotonic()
    try:
        result = await _original_call_tool(name, arguments)
        elapsed = time.monotonic() - t0
        logger.info("%s(%s) → OK (%.1fs)", name, args_summary, elapsed)
        return result
    except Exception:
        elapsed = time.monotonic() - t0
        logger.exception("%s(%s) → ERROR (%.1fs)", name, args_summary, elapsed)
        raise


mcp.call_tool = _logged_call_tool  # type: ignore[method-assign]


def main() -> None:
    """Start the MCP server with Streamable HTTP transport."""
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s  %(levelname)-5s  %(name)s  %(message)s",
        datefmt="%H:%M:%S",
    )
    mcp.run(transport="streamable-http")


if __name__ == "__main__":
    main()
