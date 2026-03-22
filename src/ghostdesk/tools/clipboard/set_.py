# Copyright (c) 2026 YV17 — MIT License
"""Clipboard set tool — write text to the clipboard via xclip."""

import asyncio

from mcp.server.fastmcp import FastMCP


async def set_clipboard(text: str) -> str:
    """Write text to the clipboard. Use with press_key("ctrl+v") to paste."""
    proc = await asyncio.create_subprocess_exec(
        "xclip", "-selection", "clipboard", "-i",
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    _, stderr = await asyncio.wait_for(proc.communicate(text.encode()), timeout=5.0)

    if proc.returncode != 0:
        raise RuntimeError(f"xclip failed: {stderr.decode().strip()}")

    return f"Clipboard set ({len(text)} characters)"


def register(mcp: FastMCP) -> None:
    """Register the set_clipboard tool."""
    mcp.tool()(set_clipboard)
