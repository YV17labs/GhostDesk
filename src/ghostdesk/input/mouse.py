# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Mouse control tools — driven by the Wayland virtual-pointer protocol.

All pointer actions go through the singleton :class:`WaylandInput`,
which keeps one persistent Wayland connection open and reuses a single
``zwlr_virtual_pointer_v1`` for the whole process lifetime.
"""

from __future__ import annotations

from mcp.server.fastmcp import Context

from ghostdesk.input._wayland import Button, ScrollDirection, get_wayland_input
from ghostdesk.input.feedback import (
    build_feedback,
    capture_before,
    poll_for_change,
    warn_on_miss,
)


async def mouse_click(
    x: int, y: int, button: Button = "left", ctx: Context | None = None,
) -> dict:
    """Click at screen coordinates. Use coordinates from screen_shot() or inspect().

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the 200x200 px zone around the click visibly
      changed within 2 s. If false the click likely missed its target —
      retry with adjusted coordinates or take a new screen_shot().
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    wl = await get_wayland_input()
    await wl.move(x, y)
    region, before = await capture_before(x, y)
    await wl.click(button)
    result = await poll_for_change(region, before)
    feedback = build_feedback(f"Clicked {button} at ({x}, {y})", result)
    await warn_on_miss(ctx, feedback)
    return feedback


async def mouse_double_click(
    x: int, y: int, button: Button = "left", ctx: Context | None = None,
) -> dict:
    """Double-click at screen coordinates. Use for opening files or selecting words.

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the 200x200 px zone around the click visibly
      changed within 2 s. If false the click likely missed its target.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    wl = await get_wayland_input()
    await wl.move(x, y)
    region, before = await capture_before(x, y)
    await wl.click(button)
    await wl.click(button)
    result = await poll_for_change(region, before)
    feedback = build_feedback(f"Double-clicked {button} at ({x}, {y})", result)
    await warn_on_miss(ctx, feedback)
    return feedback


async def mouse_drag(
    from_x: int, from_y: int, to_x: int, to_y: int,
    button: Button = "left",
    ctx: Context | None = None,
) -> dict:
    """Drag from one position to another. Use for selecting text, moving items, or resizing.

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the 200x200 px zone around the drop point
      visibly changed within 2 s.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    wl = await get_wayland_input()
    region, before = await capture_before(to_x, to_y)
    await wl.drag(from_x, from_y, to_x, to_y, button)
    result = await poll_for_change(region, before)
    feedback = build_feedback(f"Dragged from ({from_x}, {from_y}) to ({to_x}, {to_y})", result)
    await warn_on_miss(ctx, feedback)
    return feedback


async def mouse_scroll(
    x: int, y: int, direction: ScrollDirection = "down", amount: int = 3,
    ctx: Context | None = None,
) -> dict:
    """Scroll at a position. direction: up/down/left/right. amount: number of scroll steps (max 5).

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the 200x200 px zone around the scroll point
      visibly changed within 2 s. If false the page may already be at
      the scroll boundary.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    amount = max(1, min(5, int(amount)))
    wl = await get_wayland_input()

    await wl.move(x, y)
    region, before = await capture_before(x, y)
    await wl.scroll(direction, amount)
    result = await poll_for_change(region, before)
    feedback = build_feedback(f"Scrolled {direction} {amount} clicks at ({x}, {y})", result)
    await warn_on_miss(ctx, feedback)
    return feedback
