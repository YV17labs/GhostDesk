# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Clipboard get tool — read clipboard content via xclip."""

from ghostdesk._cmd import run


async def get_clipboard() -> str:
    """Read the current clipboard text."""
    try:
        return await run(["xclip", "-selection", "clipboard", "-o"])
    except RuntimeError:
        return ""
