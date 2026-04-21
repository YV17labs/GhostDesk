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


async def mouse_move(x: int, y: int, ctx: Context | None = None) -> dict:
    """Move the cursor to (x, y) without pressing any button.

    Use this to trigger hover-only UI reactions: dropdown menus that appear
    on mouse-over (e.g. Gmail action bar), tooltips, CSS ``:hover`` states,
    or any element that reveals itself when the pointer enters its bounds.

    Take a screen_shot() first to get the coordinates, then call this tool.
    If the target menu or tooltip does not appear, the element may require a
    click instead — fall back to ``mouse_click``.

    Feedback dict:
    - action: description of what was performed.
    - screen_changed: whether the 200×200 px zone centred on the target
      visibly changed within 2 s. ``false`` is expected for elements that
      have no hover effect — it is not an error.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    region, before = await capture_before(x, y)
    wl = await get_wayland_input()
    await wl.move(x, y)
    result = await poll_for_change(region, before)
    feedback = build_feedback(f"Moved cursor to ({x}, {y})", result)
    await warn_on_miss(ctx, feedback)
    return feedback


async def mouse_click(
    x: int, y: int, button: Button = "left", ctx: Context | None = None,
) -> dict:
    """Click once at screen coordinates (pixels from the last screen_shot()).

    A ``screen_changed: false`` result means the click had no visible
    effect. Do not retry the same coordinates — the target probably
    moved (page scrolled, dialog opened) or was never where you thought;
    take a new screen_shot() and recompute.

    Feedback dict:
    - action: description of what was performed.
    - screen_changed: whether the 200×200 px zone centred on the click
      visibly changed within 2 s.
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
    """Double-click at screen coordinates. Standard use cases: open a file or
    folder in a file manager, select an entire word in editable text.

    Feedback dict matches ``mouse_click``.
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
    """Drag from one point to another while holding ``button``. Standard use
    cases: select a range of text, move a window or item, resize from a
    corner, draw in a canvas.

    For selecting text, ``mouse_click(start) + key_press("shift+end")`` (or
    any shift+navigation) is often more reliable than a pixel-precise drag.

    Feedback dict matches ``mouse_click`` (change zone centred on the drop
    point).
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
    """Scroll the region under (x, y). ``direction`` is up/down/left/right,
    ``amount`` is the number of wheel notches (clamped to [1, 5] per call —
    chain multiple calls for long pages).

    A ``screen_changed: false`` result typically means the page is already
    at the scroll boundary — there is nothing more to reveal in that
    direction.

    Feedback dict matches ``mouse_click``.
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
