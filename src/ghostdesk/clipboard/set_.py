# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Clipboard set tool — write text to the clipboard via wl-copy (Wayland)."""

import asyncio


async def set_clipboard(text: str) -> str:
    """Write text to the clipboard. Use with press_key("ctrl+v") to paste."""
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

    return f"Clipboard set ({len(text)} characters)"
