# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Clipboard set tool — write text to the clipboard via xclip."""

import asyncio

from mcp.server.fastmcp import FastMCP


async def set_clipboard(text: str) -> str:
    """Write text to the clipboard. Use with press_key("ctrl+v") to paste."""
    proc = await asyncio.create_subprocess_exec(
        "xclip", "-selection", "clipboard", "-i",
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.DEVNULL,
        stderr=asyncio.subprocess.PIPE,
    )
    assert proc.stdin is not None
    proc.stdin.write(text.encode())
    proc.stdin.close()
    await asyncio.wait_for(proc.stdin.wait_closed(), timeout=5.0)

    # xclip forks a background process to serve the clipboard;
    # don't wait for it to exit — just verify stdin was accepted.

    return f"Clipboard set ({len(text)} characters)"


def register(mcp: FastMCP) -> None:
    """Register the set_clipboard tool."""
    mcp.tool()(set_clipboard)
