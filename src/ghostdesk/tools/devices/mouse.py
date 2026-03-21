# Copyright (c) 2026 YV17 — MIT License
"""Mouse control tools — with optional human-like movement."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.utils.cmd import run
from ghostdesk.utils.humanizer import get_cursor_position, human_move

_BUTTON_MAP = {"left": "1", "middle": "2", "right": "3"}


async def _move_to(x: int, y: int, humanize: bool) -> None:
    """Move cursor to (x, y) — Bézier trajectory or instant teleport."""
    if humanize:
        cx, cy = await get_cursor_position()
        await human_move(cx, cy, x, y)
    else:
        await run(["xdotool", "mousemove", str(x), str(y)])


async def mouse_move(x: int, y: int, humanize: bool = True) -> str:
    """Move the mouse cursor to absolute screen coordinates (x, y).

    Args:
        x: Horizontal pixel position.
        y: Vertical pixel position.
        humanize: If True (default), move along a realistic curve.
                  Set to False for instant teleport.
    """
    await _move_to(x, y, humanize)
    return f"Moved to ({x}, {y})"


async def mouse_click(x: int, y: int, button: str = "left", humanize: bool = True) -> str:
    """Click at screen coordinates (x, y).

    Always take a screenshot after clicking to verify the click had an effect.

    Args:
        x: Horizontal pixel position.
        y: Vertical pixel position.
        button: One of 'left', 'middle', 'right'. Defaults to 'left'.
        humanize: If True (default), move with a realistic curve before clicking.
    """
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(x, y, humanize)
    await run(["xdotool", "click", btn])
    return f"Clicked {button} at ({x}, {y})"


async def mouse_double_click(x: int, y: int, button: str = "left", humanize: bool = True) -> str:
    """Double-click at screen coordinates (x, y).

    Args:
        x: Horizontal pixel position.
        y: Vertical pixel position.
        button: One of 'left', 'middle', 'right'. Defaults to 'left'.
        humanize: If True (default), move with a realistic curve first.
    """
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(x, y, humanize)
    await run(["xdotool", "click", "--repeat", "2", "--delay", "100", btn])
    return f"Double-clicked {button} at ({x}, {y})"


async def mouse_drag(
    from_x: int, from_y: int, to_x: int, to_y: int,
    button: str = "left", humanize: bool = True,
) -> str:
    """Drag the mouse from one position to another.

    Args:
        from_x: Starting horizontal position.
        from_y: Starting vertical position.
        to_x: Ending horizontal position.
        to_y: Ending vertical position.
        button: One of 'left', 'middle', 'right'. Defaults to 'left'.
        humanize: If True (default), drag along a realistic curve.
    """
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(from_x, from_y, humanize)
    await run(["xdotool", "mousedown", btn])
    await _move_to(to_x, to_y, humanize)
    await run(["xdotool", "mouseup", btn])
    return f"Dragged from ({from_x}, {from_y}) to ({to_x}, {to_y})"


async def mouse_scroll(x: int, y: int, direction: str = "down", amount: int = 3, humanize: bool = True) -> str:
    """Scroll the mouse wheel at a given position.

    Use this to reveal content not visible on screen.

    Args:
        x: Horizontal pixel position.
        y: Vertical pixel position.
        direction: One of 'up', 'down', 'left', 'right'. Defaults to 'down'.
        amount: Number of scroll clicks. Defaults to 3.
        humanize: If True (default), move to position with a realistic curve first.
    """
    scroll_map = {"up": "4", "down": "5", "left": "6", "right": "7"}
    btn = scroll_map.get(direction, "5")
    await _move_to(x, y, humanize)
    await run(["xdotool", "click", "--repeat", str(amount), "--delay", "50", btn])
    return f"Scrolled {direction} {amount} clicks at ({x}, {y})"


def register(mcp: FastMCP) -> None:
    """Register mouse-related tools on the MCP server."""
    mcp.tool()(mouse_move)
    mcp.tool()(mouse_click)
    mcp.tool()(mouse_double_click)
    mcp.tool()(mouse_drag)
    mcp.tool()(mouse_scroll)
