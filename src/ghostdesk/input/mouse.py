# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Mouse control tools — with optional human-like movement and visual feedback."""

from ghostdesk._cmd import run
from ghostdesk.input.feedback import build_feedback, capture_before, poll_for_change
from ghostdesk._cursor import get_cursor_position
from ghostdesk.input.humanizer import human_move

_BUTTON_MAP = {"left": "1", "middle": "2", "right": "3"}


async def _move_to(x: int, y: int, humanize: bool) -> None:
    """Move cursor to (x, y) — Bézier trajectory or instant teleport."""
    if humanize:
        cx, cy = await get_cursor_position()
        await human_move(cx, cy, x, y)
    else:
        await run(["xdotool", "mousemove", str(x), str(y)])


async def mouse_click(x: int, y: int, button: str = "left", humanize: bool = True) -> dict:
    """Click at screen coordinates. Use coordinates from screenshot() or inspect().

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the 200×200 px zone around the click visibly
      changed within 2 s. If false the click likely missed its target —
      retry with adjusted coordinates or take a new screenshot.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(x, y, humanize)
    region, before = await capture_before(x, y)
    await run(["xdotool", "click", btn])
    result = await poll_for_change(region, before)
    return build_feedback(f"Clicked {button} at ({x}, {y})", result)


async def mouse_double_click(x: int, y: int, button: str = "left", humanize: bool = True) -> dict:
    """Double-click at screen coordinates. Use for opening files or selecting words.

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the 200×200 px zone around the click visibly
      changed within 2 s. If false the click likely missed its target.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(x, y, humanize)
    region, before = await capture_before(x, y)
    await run(["xdotool", "click", "--repeat", "2", "--delay", "100", btn])
    result = await poll_for_change(region, before)
    return build_feedback(f"Double-clicked {button} at ({x}, {y})", result)


async def mouse_drag(
    from_x: int, from_y: int, to_x: int, to_y: int,
    button: str = "left", humanize: bool = True,
) -> dict:
    """Drag from one position to another. Use for selecting text, moving items, or resizing.

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the 200×200 px zone around the drop point
      visibly changed within 2 s.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    btn = _BUTTON_MAP.get(button, "1")
    await _move_to(from_x, from_y, humanize)
    region, before = await capture_before(to_x, to_y)
    await run(["xdotool", "mousedown", btn])
    await _move_to(to_x, to_y, humanize)
    await run(["xdotool", "mouseup", btn])
    result = await poll_for_change(region, before)
    return build_feedback(f"Dragged from ({from_x}, {from_y}) to ({to_x}, {to_y})", result)


async def mouse_scroll(x: int, y: int, direction: str = "down", amount: int = 3, humanize: bool = True) -> dict:
    """Scroll at a position. direction: up/down/left/right. amount: number of scroll steps (max 5).

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the 200×200 px zone around the scroll point
      visibly changed within 2 s. If false the page may already be at
      the scroll boundary.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    scroll_map = {"up": "4", "down": "5", "left": "6", "right": "7"}
    btn = scroll_map.get(direction, "5")
    await _move_to(x, y, humanize)
    region, before = await capture_before(x, y)
    await run(["xdotool", "click", "--repeat", str(amount), "--delay", "50", btn])
    result = await poll_for_change(region, before)
    return build_feedback(f"Scrolled {direction} {amount} clicks at ({x}, {y})", result)
