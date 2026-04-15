# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Clipboard get tool — read clipboard content via wl-paste (Wayland)."""

from ghostdesk._cmd import run


async def clipboard_get() -> str:
    """Read the current clipboard text."""
    try:
        return await run(["wl-paste", "--no-newline"])
    except RuntimeError:
        return ""
