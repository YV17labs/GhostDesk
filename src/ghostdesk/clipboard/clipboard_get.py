# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Clipboard get tool — read clipboard content via wl-paste (Wayland)."""

from ghostdesk._cmd import run


async def clipboard_get() -> str:
    """Read the current system clipboard as text.

    Returns the empty string if the clipboard is empty or holds
    non-text content.

    Standard use cases: grab text the user just copied (to act on it or
    quote it back), fetch the output a previous CLI action was piped
    into the clipboard, or transfer a selection between two apps
    without traversing the filesystem.
    """
    try:
        return await run(["wl-paste", "--no-newline"])
    except RuntimeError:
        return ""
