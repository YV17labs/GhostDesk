# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Clipboard get tool — read clipboard content via wl-paste (Wayland)."""

from ghostdesk._cmd import run


async def clipboard_get() -> str:
    """Read the current clipboard text."""
    try:
        return await run(["wl-paste", "--no-newline"])
    except RuntimeError:
        return ""
