# Copyright (c) 2026 YV17 — MIT License
"""GhostDesk — MCP server entry point."""

import logging
import os
import time

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools import accessibility, keyboard, mouse, screenshot, shell

logger = logging.getLogger("ghostdesk")

_INSTRUCTIONS = """
You control a virtual Linux desktop (X11, display :99, resolution 1280×800).

## Tools

### Accessibility (preferred — fast, no vision needed)
- list_elements(role?) — list interactive UI elements with names and coordinates.
- click_element(text, role?) — find an element by name and click it.

### Vision (fallback — when accessibility tree is insufficient)
- screenshot() — capture the screen with a red crosshair showing cursor position.
- mouse_click(x, y) — click at exact screen coordinates.

### Input
- type_text(text) — type in the focused field.
- press_key(keys) — press a key combination (e.g. "ctrl+a", "Return").

### System
- exec(command) — run a shell command, returns stdout/stderr/returncode.
- launch(command) — launch a GUI app (fire-and-forget), returns immediately.
- wait(milliseconds) — pause between actions.

## Workflow
1. launch("firefox https://...") — open a website
2. wait(3000) — let the page load
3. list_elements() — discover what is on screen
4. click_element("Inbox") — click by name (preferred over coordinates)
5. list_elements() — verify the result
6. screenshot() — only if you need to see the visual layout

## Rules
- **Prefer list_elements + click_element over screenshot + mouse_click(x, y).**
  Accessibility tools are faster and more reliable than visual coordinate guessing.
- Use screenshot() when you need the visual layout, or when the accessibility
  tree does not expose the element you need.
- Use launch() for GUI apps (firefox, gedit, vlc...). Use exec() for CLI commands.
- After every action, verify the effect (list_elements or screenshot).
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
accessibility.register(mcp)

# Wrap call_tool to log every tool invocation with name, args, and duration.
# FastMCP's _setup_handlers captures a reference to the original call_tool before
# our registration code runs. Direct assignment (mcp.call_tool = wrapper) doesn't
# propagate because internal code has already bound the original. We use the
# lowlevel _mcp_server.call_tool(validate_input=False) decorator to properly hook
# the internal call path. This is a deliberate trade-off: accessing _mcp_server
# (private) buys us correct behavior without relying on unstable public APIs.
_original_call_tool = mcp.call_tool


async def _logged_call_tool(name: str, arguments: dict) -> object:
    # Defer args_summary construction to lazy formatting; avoid materializing large
    # argument values (e.g., screenshots) unless logging is actually at INFO level.
    def format_args() -> str:
        return ", ".join(
            f"{k}={repr(v)[:80]}"  # Truncate repr to avoid memory churn on large values
            for k, v in arguments.items()
        )

    t0 = time.monotonic()
    try:
        result = await _original_call_tool(name, arguments)
        elapsed = time.monotonic() - t0
        logger.info("%s(%s) → OK (%.1fs)", name, format_args(), elapsed)
        return result
    except Exception:
        elapsed = time.monotonic() - t0
        logger.exception("%s(%s) → ERROR (%.1fs)", name, format_args(), elapsed)
        raise


mcp._mcp_server.call_tool(validate_input=False)(_logged_call_tool)


def main() -> None:
    """Start the MCP server with Streamable HTTP transport."""
    # Ensure UTF-8 locale for proper character input
    os.environ.setdefault("LC_ALL", "en_US.utf8")
    os.environ.setdefault("LANG", "en_US.utf8")

    # Unify log format across all sources (app, MCP SDK's RichHandler, uvicorn).
    # 1) Replace the RichHandler that FastMCP.__init__ installed on the root logger.
    # 2) Patch uvicorn's LOGGING_CONFIG so its dictConfig call at startup uses our
    #    format instead of its own — setting handlers before mcp.run() is futile
    #    because uvicorn's Config.configure_logging() overwrites them via dictConfig.
    import uvicorn.config

    LOG_FMT = "%(asctime)s  %(levelname)-5s  %(name)s  %(message)s"
    LOG_DATEFMT = "%H:%M:%S"

    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter(LOG_FMT, datefmt=LOG_DATEFMT))

    root = logging.getLogger()
    root.handlers = [handler]
    root.setLevel(logging.INFO)

    fmt_cfg = {"fmt": LOG_FMT, "datefmt": LOG_DATEFMT}
    if "formatters" not in uvicorn.config.LOGGING_CONFIG:
        logger.warning(
            "uvicorn.config.LOGGING_CONFIG missing 'formatters' key; "
            "log format may not be unified across all output"
        )
    else:
        for formatter in uvicorn.config.LOGGING_CONFIG["formatters"].values():
            formatter.update(fmt_cfg)

    mcp.run(transport="streamable-http")


if __name__ == "__main__":
    main()
