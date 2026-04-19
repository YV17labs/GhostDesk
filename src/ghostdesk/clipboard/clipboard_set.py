# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Clipboard set tool — write text to the clipboard via wl-copy (Wayland)."""

import asyncio

from mcp.server.fastmcp import Context


async def clipboard_set(text: str, ctx: Context | None = None) -> str:
    """Write text to the system clipboard.

    The canonical pattern is ``clipboard_set(text)`` followed by
    ``key_press("ctrl+v")`` in the target app — the fastest and most
    reliable way to place more than a sentence or two into any editable
    field. Bypasses autocomplete and autocorrect, and does not race
    with the app's own key handlers the way ``key_type`` can.

    Returns a short confirmation string.
    """
    # wl-copy reads stdin, then forks a background daemon that keeps
    # serving the clipboard content for other apps.  We must wait for
    # the PARENT process to exit (which happens just after the fork)
    # so we know the clipboard has been set — but we must NOT use
    # communicate(), which waits for stdout/stderr to close and those
    # pipes are inherited by the daemon child, causing a timeout.
    proc = await asyncio.create_subprocess_exec(
        "wl-copy",
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.DEVNULL,
        stderr=asyncio.subprocess.DEVNULL,
    )
    assert proc.stdin is not None
    proc.stdin.write(text.encode())
    proc.stdin.close()
    await asyncio.wait_for(proc.wait(), timeout=5.0)

    message = f"Clipboard set ({len(text)} characters)"
    if ctx is not None:
        await ctx.info(message)
    return message
