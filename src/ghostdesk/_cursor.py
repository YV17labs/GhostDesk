# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Internal cursor position tracking.

Wayland intentionally does not expose the global cursor position to
user-space tools, so we mirror the last position we asked the virtual
pointer to move to. ``WaylandInput.move()`` updates this on every
successful motion event.
"""

from __future__ import annotations

from ghostdesk._coords import SCREEN_HEIGHT, SCREEN_WIDTH

_cursor_x: int | None = None
_cursor_y: int | None = None


def get_cursor_position() -> tuple[int, int]:
    """Return the last known cursor (x, y) in real screen pixels.

    Defaults to the screen center on first call.
    """
    global _cursor_x, _cursor_y
    if _cursor_x is None or _cursor_y is None:
        _cursor_x, _cursor_y = SCREEN_WIDTH // 2, SCREEN_HEIGHT // 2
    return _cursor_x, _cursor_y


def set_cursor_position(x: int, y: int) -> None:
    """Record that the cursor is now at (x, y)."""
    global _cursor_x, _cursor_y
    _cursor_x = int(x)
    _cursor_y = int(y)
