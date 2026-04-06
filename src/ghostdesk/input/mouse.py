# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Mouse control tools — with optional human-like movement."""

from ghostdesk._cmd import run
from ghostdesk.input.humanizer import get_cursor_position, human_move

_BUTTON_MAP = {"left": "1", "middle": "2", "right": "3"}


async def _move_to(x: int, y: int, humanize: bool) -> None:
    """Move cursor to (x, y) — Bézier trajectory or instant teleport."""
    if humanize:
        cx, cy = await get_cursor_position()
        await human_move(cx, cy, x, y)
    else:
        await run(["xdotool", "mousemove", str(x), str(y)])


async def mouse_click(x: int, y: int, button: str = "left", humanize: bool = True) -> str:
    """Click at screen coordinates. Use coordinates from screenshot() or inspect()."""
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(x, y, humanize)
    await run(["xdotool", "click", btn])
    return f"Clicked {button} at ({x}, {y})"


async def mouse_double_click(x: int, y: int, button: str = "left", humanize: bool = True) -> str:
    """Double-click at screen coordinates. Use for opening files or selecting words."""
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(x, y, humanize)
    await run(["xdotool", "click", "--repeat", "2", "--delay", "100", btn])
    return f"Double-clicked {button} at ({x}, {y})"


async def mouse_drag(
    from_x: int, from_y: int, to_x: int, to_y: int,
    button: str = "left", humanize: bool = True,
) -> str:
    """Drag from one position to another. Use for selecting text, moving items, or resizing."""
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(from_x, from_y, humanize)
    await run(["xdotool", "mousedown", btn])
    await _move_to(to_x, to_y, humanize)
    await run(["xdotool", "mouseup", btn])
    return f"Dragged from ({from_x}, {from_y}) to ({to_x}, {to_y})"


async def mouse_scroll(x: int, y: int, direction: str = "down", amount: int = 3, humanize: bool = True) -> str:
    """Scroll at a position. direction: up/down/left/right. amount: number of scroll steps (max 5)."""
    scroll_map = {"up": "4", "down": "5", "left": "6", "right": "7"}
    btn = scroll_map.get(direction, "5")
    await _move_to(x, y, humanize)
    await run(["xdotool", "click", "--repeat", str(amount), "--delay", "50", btn])
    return f"Scrolled {direction} {amount} clicks at ({x}, {y})"
