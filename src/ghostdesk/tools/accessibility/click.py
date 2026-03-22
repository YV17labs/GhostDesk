# Copyright (c) 2026 YV17 — MIT License
"""Accessibility click tool — bridges AT-SPI element discovery with xdotool click.

This tool intentionally crosses the channel boundary: it uses the accessibility
tree (AT-SPI) to locate an element by name, then performs the physical click
via xdotool with optional human-like mouse movement.  The bridge is isolated
in this file so the rest of the accessibility channel remains AT-SPI–pure.
"""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.accessibility._client import run_atspi
from ghostdesk.utils.cmd import run
from ghostdesk.utils.humanizer import get_cursor_position, human_move


async def click_element(
    text: str,
    role: str | None = None,
    humanize: bool = True,
) -> str:
    """Find a UI element by name and click it."""
    args = [text]
    if role:
        args.extend(["--role", role])

    try:
        match = await run_atspi("find", args)
    except RuntimeError:
        match = {}

    if "center_x" not in match:
        return (
            f"No element found matching '{text}'. "
            f"Try read_screen() to see available elements, "
            f"or use mouse_click(x, y) with coordinates from screenshot()."
        )

    x, y = match["center_x"], match["center_y"]

    if humanize:
        cx, cy = await get_cursor_position()
        await human_move(cx, cy, x, y)
    else:
        await run(["xdotool", "mousemove", str(x), str(y)])

    await run(["xdotool", "click", "1"])

    return (
        f"Clicked '{match['name']}' ({match['role']}) "
        f"at ({x}, {y})"
    )


def register(mcp: FastMCP) -> None:
    """Register the click_element tool."""
    mcp.tool()(click_element)
