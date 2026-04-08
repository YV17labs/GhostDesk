# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Cursor position utility — shared across input and screen domains."""

from ghostdesk._cmd import run


async def get_cursor_position() -> tuple[int, int]:
    """Return current cursor (x, y) from xdotool."""
    output = await run(["xdotool", "getmouselocation"])
    # Format: "x:123 y:456 screen:0 window:789"
    parts = dict(p.split(":") for p in output.split())
    return int(parts["x"]), int(parts["y"])
