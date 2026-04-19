# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Clipboard resource — read-only view of the current clipboard text."""

from ghostdesk.clipboard.clipboard_get import clipboard_get


async def clipboard_resource() -> str:
    """Current clipboard text — same content as the ``clipboard_get`` tool."""
    return await clipboard_get()
