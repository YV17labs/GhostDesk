# Copyright (c) 2026 YV17 — MIT License
"""Clipboard get tool — read clipboard content via xclip."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.utils.cmd import run


async def get_clipboard() -> str:
    """Read the current content of the system clipboard.

    Returns the text currently stored in the clipboard. Useful for
    extracting text that was copied by the user or by a previous action.
    """
    try:
        return await run(["xclip", "-selection", "clipboard", "-o"])
    except RuntimeError:
        return ""


def register(mcp: FastMCP) -> None:
    """Register the get_clipboard tool."""
    mcp.tool()(get_clipboard)
